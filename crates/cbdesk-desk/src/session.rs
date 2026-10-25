// SPDX-License-Identifier: AGPL-3.0-or-later

//! One granted session: capture or a queued picture, then the side streams.
//!
//! Tests queue a 48×16 motion pair and set `preview_hold` so this loop does
//! not encode a full display. Production uses a submitted frame (Android,
//! after the system consent dialog) and otherwise `capture_picture`. A
//! missing display is skipped. It is not replaced with a synthetic barcode.
//! Input injection runs only when the grant includes Input and the test
//! record flag is off.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cbdesk_audio::{CapturePump, PacketCodec, Speaker};
use cbdesk_capture::{capture_picture, crop_even, Picture};
use cbdesk_codec::{
    composite_vp8, vp8_is_inter, BitrateControl, DamageEncoder, DamagePacket, PlacedFrame,
    Vp8StreamDecoder,
};
use cbdesk_fs::{hash_bytes, plan_chunks, IncomingFile, CHUNK_DEFAULT};
use cbdesk_input::inject_os;
use cbdesk_net::{
    recv_clipboard_text, recv_uni, send_audio_packet, send_clipboard_text, send_file_chunk,
    send_input_event, ControllerGrant, HostLink, PeerEvent, QuicConn, UniBody,
};
use cbdesk_proto::{
    prepare_clipboard_text, ClipboardText, Codec, FileChunk, InputBody, InputEvent, KeyEvent,
    Permission, PointerEvent, ProtoError, UnknownFields, VideoFrameHeader,
};
use tokio::sync::{mpsc, oneshot};

use crate::error::DeskError;
use crate::preview::PreviewPacket;
use crate::state::{self, Inner};

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
const FILE_MAX: usize = 32 * 1024 * 1024;
const AUDIO_MAX: usize = 4 * 1024;

pub(crate) type Reply = oneshot::Sender<Result<(), String>>;

/// A command from the shell into the session task.
pub(crate) enum Cmd {
    /// Pointer sample. The host injects it only after an Input grant.
    Pointer {
        /// Horizontal position.
        x: i32,
        /// Vertical position.
        y: i32,
        /// Button mask.
        buttons: u32,
        /// Wheel ticks.
        wheel: i32,
        /// Completes when the event has been written.
        reply: Reply,
    },
    /// Key transition.
    Key {
        /// Key code.
        keycode: u32,
        /// Press or release.
        down: bool,
        /// Modifier mask.
        modifiers: u32,
        /// Completes when the event has been written.
        reply: Reply,
    },
    /// One whole file, already in memory. The name is jailed by the receiver.
    File {
        /// Single path component.
        name: String,
        /// File bytes. Larger than 32 MiB is refused.
        bytes: Vec<u8>,
        /// Completes after the peer acks the last chunk.
        reply: Reply,
    },
    /// Clipboard text. Empty clears.
    Clipboard {
        /// UTF-8 text.
        text: String,
        /// Completes when the text has been written.
        reply: Reply,
    },
    /// One Opus packet from a test, or from a pump the shell already encoded.
    Audio {
        /// Packet bytes.
        bytes: Vec<u8>,
        /// Completes when the packet has been written.
        reply: Reply,
    },
    /// Open or drop the default microphone.
    Mic {
        /// True opens the default input.
        enabled: bool,
        /// Completes when the device has been opened or dropped.
        reply: Reply,
    },
}

pub(crate) async fn host_session(
    inner: &Mutex<Inner>,
    mut link: HostLink,
    mut cmds: mpsc::UnboundedReceiver<Cmd>,
    mut cancel: oneshot::Receiver<()>,
    session_id: u64,
) -> Result<(), DeskError> {
    let mut pump = Pump::new();
    let mut uploads = HashMap::new();
    let mut heard = Heard::open();
    loop {
        let pause = tokio::time::sleep(pump.wait_for());
        tokio::pin!(pause);
        tokio::select! {
            biased;
            _ = &mut cancel => break,
            cmd = cmds.recv() => {
                match cmd {
                    Some(cmd) => host_cmd(&mut link, cmd).await,
                    None => break,
                }
            }
            event = link.next_peer_event() => {
                if !host_event(inner, &mut link, &mut pump, &mut uploads, &mut heard, event).await? {
                    break;
                }
            }
            _ = &mut pause => send_due(inner, &mut link, &mut pump).await?,
        }
    }
    finish_host(inner)?;
    clear_session(inner, session_id);
    Ok(())
}

pub(crate) async fn viewer_session(
    inner: Arc<Mutex<Inner>>,
    conn: QuicConn,
    grant: ControllerGrant,
    mut cmds: mpsc::UnboundedReceiver<Cmd>,
    mut cancel: oneshot::Receiver<()>,
    mut ready: Option<oneshot::Sender<Result<(), String>>>,
    session_id: u64,
) {
    let mut viewer = Viewer {
        conn,
        grant,
        decoder: Vp8StreamDecoder::new(),
        canvas: None,
        inner: Arc::clone(&inner),
        mic: None,
    };
    loop {
        let tick = viewer.mic.is_some();
        let pause = tokio::time::sleep(Duration::from_millis(if tick { 20 } else { 400 }));
        tokio::pin!(pause);
        tokio::select! {
            biased;
            _ = &mut cancel => break,
            cmd = cmds.recv() => {
                match cmd {
                    Some(cmd) => {
                        let _ = viewer_cmd(&mut viewer, &mut ready, cmd).await;
                    }
                    None => break,
                }
            }
            body = recv_uni(&viewer.conn) => {
                if !take_body(&mut viewer, &mut ready, body) {
                    break;
                }
            }
            clip = recv_clipboard_text(&viewer.conn) => {
                if let Ok(text) = clip {
                    let _ = remember_clipboard(&viewer.inner, &text.text);
                }
            }
            _ = &mut pause, if tick => {
                if flush_mic(&viewer).await.is_err() {
                    break;
                }
            }
        }
    }
    if let Some(tx) = ready.take() {
        let _ = tx.send(Err("session ended before an inter frame".into()));
    }
    let _ = viewer.grant.release().await;
    clear_session(&inner, session_id);
}

struct Viewer {
    conn: QuicConn,
    grant: ControllerGrant,
    decoder: Vp8StreamDecoder,
    canvas: Option<Picture>,
    inner: Arc<Mutex<Inner>>,
    mic: Option<CapturePump>,
}

struct Heard {
    speaker: Option<Speaker>,
    playback: Option<PacketCodec>,
}

impl Heard {
    fn open() -> Self {
        let speaker = Speaker::open_default().ok();
        let playback = speaker.as_ref().and_then(|_| PacketCodec::new().ok());
        Self { speaker, playback }
    }
}

struct Pump {
    encoder: DamageEncoder,
    rate: BitrateControl,
    skip_next: bool,
    frame_no: u64,
    next_at: Option<Instant>,
}

impl Pump {
    fn new() -> Self {
        Self {
            encoder: DamageEncoder::new(),
            rate: BitrateControl::live(),
            skip_next: false,
            frame_no: 0,
            next_at: None,
        }
    }
}

impl Pump {
    fn wait_for(&self) -> Duration {
        self.next_at
            .map(|at| at.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::ZERO)
    }

    fn arm(&mut self, hold: bool) {
        let gap = if hold {
            Duration::from_millis(400)
        } else {
            FRAME_INTERVAL
        };
        self.next_at = Some(Instant::now() + gap);
    }
}

async fn host_cmd(link: &mut HostLink, cmd: Cmd) {
    let result = match cmd {
        Cmd::Clipboard { text, reply } => {
            let result = send_text(link, &text).await;
            let _ = reply.send(result);
            return;
        }
        other => other,
    };
    reply_err(result, "the host receives that from the controller");
}

fn reply_err(cmd: Cmd, message: &str) {
    let reply = match cmd {
        Cmd::Pointer { reply, .. }
        | Cmd::Key { reply, .. }
        | Cmd::File { reply, .. }
        | Cmd::Clipboard { reply, .. }
        | Cmd::Audio { reply, .. }
        | Cmd::Mic { reply, .. } => reply,
    };
    let _ = reply.send(Err(message.into()));
}

async fn send_text(link: &mut HostLink, text: &str) -> Result<(), String> {
    let clip = ClipboardText {
        text: text.to_string(),
        generation: 1,
        unknown: UnknownFields::new(),
    };
    link.send_clipboard(&clip)
        .await
        .map_err(|err| err.to_string())
}

async fn host_event(
    inner: &Mutex<Inner>,
    link: &mut HostLink,
    pump: &mut Pump,
    uploads: &mut HashMap<u64, IncomingFile>,
    heard: &mut Heard,
    event: Result<PeerEvent, cbdesk_net::AdmitError>,
) -> Result<bool, DeskError> {
    let event = match event {
        Ok(event) => event,
        Err(_) => return Ok(false),
    };
    match event {
        PeerEvent::Ended | PeerEvent::Control(_) => Ok(false),
        PeerEvent::Input(event) => {
            note_input(inner, &event)?;
            Ok(true)
        }
        PeerEvent::Clipboard(text) => {
            remember_clipboard(inner, &text.text)?;
            Ok(true)
        }
        PeerEvent::File(chunk) => {
            if take_file(inner, uploads, &chunk)? {
                link.ack_file().await.map_err(msg)?;
            }
            Ok(true)
        }
        PeerEvent::Audio(packet) => {
            note_audio(inner, heard, &packet)?;
            Ok(true)
        }
        PeerEvent::Feedback(feedback) => {
            pump.rate
                .observe_feedback(feedback.loss, feedback.frame_delay_ms);
            Ok(true)
        }
    }
}

fn note_input(inner: &Mutex<Inner>, event: &InputEvent) -> Result<(), DeskError> {
    let record = lock(inner)?.record_input;
    if !record {
        inject_os(event).map_err(msg)?;
    }
    let mut guard = lock(inner)?;
    guard.last_input = describe(event);
    Ok(())
}

fn describe(event: &InputEvent) -> String {
    match &event.body {
        InputBody::Pointer(pointer) => format!("pointer {} {}", pointer.x, pointer.y),
        InputBody::Key(key) => format!("key {} {}", key.keycode, u8::from(key.down)),
    }
}

fn remember_clipboard(inner: &Mutex<Inner>, text: &str) -> Result<(), DeskError> {
    let plain = match prepare_clipboard_text(text) {
        Ok(plain) => plain,
        Err(ProtoError::ClipboardSecret) => return Ok(()),
        Err(err) => return Err(msg(err)),
    };
    lock(inner)?.clipboard = plain.clone();
    copy_to_os_clipboard(&plain);
    Ok(())
}

fn copy_to_os_clipboard(text: &str) {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    if let Ok(mut board) = arboard::Clipboard::new() {
        let _ = board.set_text(text);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    let _ = text;
}

fn note_audio(inner: &Mutex<Inner>, heard: &mut Heard, packet: &[u8]) -> Result<(), DeskError> {
    {
        let mut guard = lock(inner)?;
        guard.audio_packets = guard.audio_packets.saturating_add(1);
    }
    if let (Some(speaker), Some(playback)) = (heard.speaker.as_ref(), heard.playback.as_mut()) {
        if let Ok(pcm) = playback.decode_packet(packet) {
            let _ = speaker.play(&pcm);
        }
    }
    Ok(())
}

fn take_file(
    inner: &Mutex<Inner>,
    uploads: &mut HashMap<u64, IncomingFile>,
    chunk: &FileChunk,
) -> Result<bool, DeskError> {
    if chunk.compressed {
        return Err(DeskError::Message(
            "compressed file chunks are not accepted".into(),
        ));
    }
    if let std::collections::hash_map::Entry::Vacant(entry) = uploads.entry(chunk.transfer) {
        let root = {
            let guard = lock(inner)?;
            state::files_root(&guard)
        };
        let file =
            IncomingFile::begin(&root, &chunk.path, chunk.total, chunk.chunk_size).map_err(msg)?;
        entry.insert(file);
    }
    let file = uploads
        .get_mut(&chunk.transfer)
        .ok_or_else(|| DeskError::Message("file transfer is not open".into()))?;
    file.push(chunk.offset, &chunk.data, &chunk.hash)
        .map_err(msg)?;
    if !chunk.last {
        return Ok(false);
    }
    let file = uploads
        .remove(&chunk.transfer)
        .ok_or_else(|| DeskError::Message("file transfer is not open".into()))?;
    file.finish(&chunk.root).map_err(msg)?;
    Ok(true)
}

async fn send_due(
    inner: &Mutex<Inner>,
    link: &mut HostLink,
    pump: &mut Pump,
) -> Result<(), DeskError> {
    if pump.skip_next {
        pump.skip_next = false;
        pump.arm(false);
        return Ok(());
    }
    if let Some(packet) = pop_preview(inner)? {
        let outgoing = DamagePacket {
            bytes: packet.bytes,
            keyframe: packet.keyframe,
            unchanged: false,
            origin_x: 0,
            origin_y: 0,
            width: packet.picture.width,
            height: packet.picture.height,
            region: false,
            scan_us: 0,
        };
        send_encoded(link, pump, &outgoing).await?;
        store_picture(inner, &packet.picture, !packet.keyframe)?;
        pump.arm(false);
        return Ok(());
    }
    let hold = lock(inner)?.preview_hold;
    if let Some(picture) = pop_submitted(inner)? {
        encode_and_send(inner, link, pump, picture).await?;
        pump.arm(hold);
        return Ok(());
    }
    if hold {
        pump.arm(true);
        return Ok(());
    }
    match capture_one() {
        Ok(picture) => encode_and_send(inner, link, pump, picture).await?,
        Err(err) => {
            let mut guard = lock(inner)?;
            if guard.error.is_empty() {
                guard.error = err;
            }
        }
    }
    pump.arm(false);
    Ok(())
}

fn pop_preview(inner: &Mutex<Inner>) -> Result<Option<PreviewPacket>, DeskError> {
    Ok(lock(inner)?.preview.pop_front())
}

fn pop_submitted(inner: &Mutex<Inner>) -> Result<Option<Picture>, DeskError> {
    Ok(lock(inner)?.submitted.pop_front())
}

fn capture_one() -> Result<Picture, String> {
    let picture = capture_picture().map_err(|err| err.to_string())?;
    crop_even(picture).map_err(|err| err.to_string())
}

async fn encode_and_send(
    inner: &Mutex<Inner>,
    link: &mut HostLink,
    pump: &mut Pump,
    picture: Picture,
) -> Result<(), DeskError> {
    let qindex = pump.rate.qindex();
    let started = Instant::now();
    let encoder = std::mem::take(&mut pump.encoder);
    let (encoder, packet) = encode_on_thread(encoder, picture.clone(), qindex).await?;
    pump.encoder = encoder;
    if pump.rate.observe_encode(elapsed_ms(started)) == cbdesk_codec::Pace::SkipNext {
        pump.skip_next = true;
    }
    if !packet.unchanged {
        send_encoded(link, pump, &packet).await?;
        store_picture(inner, &picture, !packet.keyframe || packet.region)?;
    }
    Ok(())
}

async fn encode_on_thread(
    encoder: DamageEncoder,
    picture: Picture,
    qindex: u8,
) -> Result<(DamageEncoder, DamagePacket), DeskError> {
    tokio::task::spawn_blocking(move || {
        let handle = std::thread::Builder::new()
            .name("cbdesk-encode".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let mut encoder = encoder;
                let encoded = encoder.encode(&picture, qindex);
                (encoder, encoded)
            })
            .map_err(msg)?;
        let (encoder, encoded) = handle
            .join()
            .map_err(|_| DeskError::Message("encode thread panicked".into()))?;
        let packet = encoded.map_err(msg)?;
        Ok((encoder, packet))
    })
    .await
    .map_err(msg)?
}

async fn send_encoded(
    link: &mut HostLink,
    pump: &mut Pump,
    packet: &DamagePacket,
) -> Result<(), DeskError> {
    pump.frame_no = pump.frame_no.saturating_add(1);
    let region = packet.region;
    let header = VideoFrameHeader {
        display_id: 1,
        codec: Codec::Vp8,
        frame_no: pump.frame_no,
        capture_ts_us: 1_000,
        keyframe: packet.keyframe && !region,
        damage_x: if region { packet.origin_x } else { 0 },
        damage_y: if region { packet.origin_y } else { 0 },
        damage_width: if region { packet.width } else { 0 },
        damage_height: if region { packet.height } else { 0 },
        unchanged: false,
        unknown: UnknownFields::new(),
    };
    link.send_view_frame(&header, &packet.bytes)
        .await
        .map_err(msg)
}

fn store_picture(inner: &Mutex<Inner>, picture: &Picture, inter: bool) -> Result<(), DeskError> {
    let rgba = picture.to_rgba().map_err(msg)?;
    let mut guard = lock(inner)?;
    guard.frame_width = picture.width;
    guard.frame_height = picture.height;
    guard.frame_is_inter = inter;
    guard.frame_rgba = rgba;
    Ok(())
}

async fn viewer_cmd(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    cmd: Cmd,
) -> Result<(), String> {
    match cmd {
        Cmd::Pointer {
            x,
            y,
            buttons,
            wheel,
            reply,
        } => {
            let result = send_pointer(viewer, ready, x, y, buttons, wheel).await;
            let _ = reply.send(result);
        }
        Cmd::Key {
            keycode,
            down,
            modifiers,
            reply,
        } => {
            let result = send_key(viewer, ready, keycode, down, modifiers).await;
            let _ = reply.send(result);
        }
        Cmd::File { name, bytes, reply } => {
            let result = send_file(viewer, ready, &name, &bytes).await;
            let _ = reply.send(result);
        }
        Cmd::Clipboard { text, reply } => {
            let result = send_clip(viewer, &text).await;
            let _ = reply.send(result);
        }
        Cmd::Audio { bytes, reply } => {
            let result = send_one_audio(viewer, &bytes).await;
            let _ = reply.send(result);
        }
        Cmd::Mic { enabled, reply } => {
            let result = set_mic(viewer, enabled);
            let _ = reply.send(result);
        }
    }
    Ok(())
}

async fn send_pointer(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    x: i32,
    y: i32,
    buttons: u32,
    wheel: i32,
) -> Result<(), String> {
    require(&viewer.grant, Permission::Input)?;
    let event = InputEvent {
        body: InputBody::Pointer(PointerEvent {
            x,
            y,
            buttons,
            wheel,
            unknown: UnknownFields::new(),
        }),
        unknown: UnknownFields::new(),
    };
    send_input_event(&viewer.conn, &event)
        .await
        .map_err(|err| err.to_string())?;
    wait_mark(
        viewer,
        ready,
        |body| matches!(body, UniBody::InputAck),
        "input ack",
    )
    .await
}

async fn send_key(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    keycode: u32,
    down: bool,
    modifiers: u32,
) -> Result<(), String> {
    require(&viewer.grant, Permission::Input)?;
    let event = InputEvent {
        body: InputBody::Key(KeyEvent {
            keycode,
            down,
            modifiers,
            unknown: UnknownFields::new(),
        }),
        unknown: UnknownFields::new(),
    };
    send_input_event(&viewer.conn, &event)
        .await
        .map_err(|err| err.to_string())?;
    wait_mark(
        viewer,
        ready,
        |body| matches!(body, UniBody::InputAck),
        "input ack",
    )
    .await
}

async fn send_file(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    require(&viewer.grant, Permission::FileWrite)?;
    if bytes.len() > FILE_MAX {
        return Err("file is larger than 32 MiB".into());
    }
    let root = {
        let guard = viewer
            .inner
            .lock()
            .map_err(|_| "desktop state lock was poisoned")?;
        state::files_root(&guard)
    };
    cbdesk_fs::resolve(&root, name).map_err(|err| err.to_string())?;
    let chunks = chunks_for(name, bytes)?;
    for chunk in &chunks {
        send_file_chunk(&viewer.conn, chunk)
            .await
            .map_err(|err| err.to_string())?;
    }
    wait_mark(
        viewer,
        ready,
        |body| matches!(body, UniBody::FileAck),
        "file ack",
    )
    .await
}

fn chunks_for(name: &str, bytes: &[u8]) -> Result<Vec<FileChunk>, String> {
    let root = hash_bytes(bytes);
    let planned = plan_chunks(bytes, CHUNK_DEFAULT).map_err(|err| err.to_string())?;
    let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut out = Vec::new();
    for piece in planned {
        out.push(FileChunk {
            transfer: 1,
            path: name.to_string(),
            offset: piece.offset,
            data: piece.data.to_vec(),
            hash: piece.hash.to_vec(),
            total,
            last: piece.last,
            root: if piece.last {
                root.to_vec()
            } else {
                Vec::new()
            },
            compressed: false,
            chunk_size: CHUNK_DEFAULT,
            unknown: UnknownFields::new(),
        });
    }
    Ok(out)
}

async fn send_clip(viewer: &Viewer, text: &str) -> Result<(), String> {
    require(&viewer.grant, Permission::Clipboard)?;
    let text = match prepare_clipboard_text(text) {
        Ok(text) => text,
        Err(ProtoError::ClipboardSecret) => return Ok(()),
        Err(err) => return Err(err.to_string()),
    };
    let clip = ClipboardText {
        text,
        generation: 1,
        unknown: UnknownFields::new(),
    };
    send_clipboard_text(&viewer.conn, &clip)
        .await
        .map_err(|err| err.to_string())
}

async fn send_one_audio(viewer: &Viewer, bytes: &[u8]) -> Result<(), String> {
    require(&viewer.grant, Permission::Audio)?;
    if bytes.is_empty() || bytes.len() > AUDIO_MAX {
        return Err("audio packet was rejected".into());
    }
    send_audio_packet(&viewer.conn, bytes)
        .await
        .map_err(|err| err.to_string())
}

fn set_mic(viewer: &mut Viewer, enabled: bool) -> Result<(), String> {
    if !enabled {
        viewer.mic = None;
        return Ok(());
    }
    require(&viewer.grant, Permission::Audio)?;
    viewer.mic = Some(CapturePump::start().map_err(|err| err.to_string())?);
    Ok(())
}

async fn flush_mic(viewer: &Viewer) -> Result<(), String> {
    let Some(mic) = &viewer.mic else {
        return Ok(());
    };
    let mut packets = Vec::new();
    while let Some(packet) = mic.pop() {
        packets.push(packet);
    }
    for packet in packets {
        send_one_audio(viewer, &packet).await?;
    }
    Ok(())
}

fn require(grant: &ControllerGrant, perm: Permission) -> Result<(), String> {
    if grant.granted.perms.contains(&perm) {
        Ok(())
    } else {
        Err(format!("host did not grant {perm:?}"))
    }
}

async fn wait_mark(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    mut done: impl FnMut(&UniBody) -> bool,
    label: &str,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {label}"));
        }
        let body = tokio::time::timeout(Duration::from_secs(10), recv_uni(&viewer.conn))
            .await
            .map_err(|_| format!("timed out waiting for {label}"))?
            .map_err(|err| err.to_string())?;
        if done(&body) {
            return Ok(());
        }
        apply_uni(viewer, ready, body).map_err(|err| err.to_string())?;
    }
}

fn take_body(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    body: Result<UniBody, cbdesk_net::NetError>,
) -> bool {
    match body {
        Ok(body) => match apply_uni(viewer, ready, body) {
            Ok(()) => true,
            Err(err) => {
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Err(err.to_string()));
                }
                false
            }
        },
        Err(_) => false,
    }
}

fn apply_uni(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    body: UniBody,
) -> Result<(), DeskError> {
    match body {
        UniBody::Video(header, payload) => apply_video(viewer, ready, &header, &payload),
        UniBody::Audio(_)
        | UniBody::File(_)
        | UniBody::Feedback(_)
        | UniBody::InputAck
        | UniBody::FileAck => Ok(()),
    }
}

fn apply_video(
    viewer: &mut Viewer,
    ready: &mut Option<oneshot::Sender<Result<(), String>>>,
    header: &VideoFrameHeader,
    payload: &[u8],
) -> Result<(), DeskError> {
    if header.codec != Codec::Vp8 {
        return Err(DeskError::Message(
            "host sent a codec this build cannot decode".into(),
        ));
    }
    let picture = composite_vp8(
        &mut viewer.canvas,
        &mut viewer.decoder,
        PlacedFrame {
            unchanged: header.unchanged,
            damage_x: header.damage_x,
            damage_y: header.damage_y,
            damage_width: header.damage_width,
            damage_height: header.damage_height,
            payload,
        },
    )
    .map_err(msg)?;
    let inter = header.damage_width > 0 || (!header.keyframe && vp8_is_inter(payload));
    store_picture(&viewer.inner, &picture, inter)?;
    if inter {
        if let Some(tx) = ready.take() {
            let _ = tx.send(Ok(()));
        }
        if let Ok(mut guard) = viewer.inner.lock() {
            guard.viewer_live = true;
        }
    }
    Ok(())
}

fn finish_host(inner: &Mutex<Inner>) -> Result<(), DeskError> {
    let mut guard = lock(inner)?;
    let _ = guard
        .session
        .close_with(cbdesk_session::CloseReason::LocalClose);
    Ok(())
}

pub(crate) fn clear_session(inner: &Mutex<Inner>, session_id: u64) {
    if let Ok(mut guard) = inner.lock() {
        if guard.session_id == session_id {
            guard.commands = None;
            guard.viewer_live = false;
        }
    }
}

pub(crate) fn install(
    inner: &Mutex<Inner>,
    cmds: mpsc::UnboundedSender<Cmd>,
    cancel: oneshot::Sender<()>,
) -> Result<u64, DeskError> {
    let mut guard = lock(inner)?;
    if let Some(old) = guard.cancel.take() {
        let _ = old.send(());
    }
    let id = guard.session_id.saturating_add(1);
    guard.session_id = id;
    guard.commands = Some((id, cmds));
    guard.cancel = Some(cancel);
    Ok(id)
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

fn lock(inner: &Mutex<Inner>) -> Result<std::sync::MutexGuard<'_, Inner>, DeskError> {
    inner
        .lock()
        .map_err(|_| DeskError::Message("desktop state lock was poisoned".into()))
}

fn msg(err: impl ToString) -> DeskError {
    DeskError::Message(err.to_string())
}

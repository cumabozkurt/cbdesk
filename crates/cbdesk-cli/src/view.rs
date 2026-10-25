// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dial a host and show its frames.
//!
//! `--ppm` writes one picture and leaves. `--window` keeps reading until the
//! window closes, and forwards the pointer and keyboard only after an Input
//! grant. `--put` uploads one file after FileWrite. `--audio` prints the
//! length of Opus packets. `--browse` finds the host on the LAN and dials
//! loopback before the multicast source address. A ticket is still only a
//! locator. Relay URLs are used only when this process was given the same URL.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cbdesk_audio::{CapturePump, PacketCodec, Speaker};
use cbdesk_capture::Picture;
use cbdesk_codec::{composite_vp8, decode_luma, PlacedFrame, Vp8StreamDecoder};
use cbdesk_crypto::Identity;
use cbdesk_discovery::ReplayCache;
use cbdesk_net::{
    connect_candidates, dial_verified_ticket, open_controller, recv_clipboard_text, recv_uni,
    send_audio_packet, send_clipboard_text, send_feedback, send_input_event, ControllerGrant,
    ControllerSession, EndpointConfig, QuicConn, QuicEndpoint, UniBody,
};
use cbdesk_proto::{
    local_capabilities, select_video_codec, AuthMethod, ClipboardText, Codec, Feedback, Hello,
    InputEvent, Permission, Platform, ProtoMessage, UnknownFields, VideoFrameHeader, PROTO_MAJOR,
    PROTO_MINOR,
};
use zeroize::Zeroize;

use crate::args::ViewArgs;
use crate::clipboard::{ClipSync, OsClipboard};
use crate::error::CliError;
use crate::hex;
use crate::identity::open_identity;
use crate::lan::{browse_peer, dial_candidates};
use crate::ppm::write_ppm;
use crate::window::{self, WindowOptions};

struct LiveView {
    /// Dropping the endpoint closes the connection, so it stays here.
    _endpoint: QuicEndpoint,
    conn: QuicConn,
    grant: ControllerGrant,
    codec: Option<Codec>,
    decoder: Vp8StreamDecoder,
    canvas: Option<Picture>,
    _identity: Identity,
    mic: Option<CapturePump>,
    speaker: Option<Speaker>,
    playback: Option<PacketCodec>,
}

/// Dial, show one frame or upload a file, then leave.
pub async fn view_remote(args: &ViewArgs) -> Result<(), CliError> {
    let mut live = connect_view(args).await?;
    let result = run_remote(&mut live, args).await;
    let _ = live.grant.release().await;
    result
}

/// Dial, then run the window on this thread until it closes.
pub fn view_window(args: &ViewArgs) -> Result<(), CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .map_err(|err| CliError::new(err.to_string()))?;
    let live = runtime.block_on(connect_view(args))?;
    let allow_input = live.grant.granted.perms.contains(&Permission::Input);
    let allow_clip = live.grant.granted.perms.contains(&Permission::Clipboard);
    let frames = Arc::new(Mutex::new(None));
    let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let ppm = args.ppm.clone();
    let pointer = args.pointer;
    let key = args.key.clone();
    let put = args.put.clone();
    let shown = Arc::clone(&frames);
    let task = runtime.spawn(async move {
        let drive = WindowDrive {
            live,
            frames: shown,
            input_rx,
            stop_rx,
            allow_input,
            allow_clip,
            ppm,
            pointer,
            key,
            put,
        };
        drive_window(drive).await
    });
    let window_result = window::run(
        frames,
        input_tx,
        stop_tx,
        WindowOptions {
            allow_input,
            any_thread: false,
            close_after: None,
        },
    );
    let net_result = runtime
        .block_on(task)
        .map_err(|err| CliError::new(err.to_string()))?;
    window_result?;
    net_result
}

struct WindowDrive {
    live: LiveView,
    frames: Arc<Mutex<Option<Picture>>>,
    input_rx: tokio::sync::mpsc::UnboundedReceiver<InputEvent>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    allow_input: bool,
    allow_clip: bool,
    ppm: Option<PathBuf>,
    pointer: Option<(i32, i32)>,
    key: Option<String>,
    put: Option<PathBuf>,
}

async fn drive_window(mut drive: WindowDrive) -> Result<(), CliError> {
    let result = drive_until_close(&mut drive).await;
    let _ = drive.live.grant.release().await;
    result
}

async fn drive_until_close(drive: &mut WindowDrive) -> Result<(), CliError> {
    if let Some(path) = drive.put.clone() {
        let mut ignored = false;
        send_while_reading(&mut drive.live, &path, &mut ignored).await?;
    }
    let mut clips = ClipSync::new();
    let mut wrote_ppm = false;
    let mut sent_script = false;
    let mut report = ReportClock::default();
    loop {
        // A microphone has to be flushed while the window is open. Clipboard
        // polling stays on the slower tick when there is no microphone.
        let tick_ms = if drive.live.mic.is_some() { 20 } else { 400 };
        let pause = tokio::time::sleep(Duration::from_millis(tick_ms));
        tokio::pin!(pause);
        tokio::select! {
            biased;
            changed = drive.stop_rx.changed() => {
                if changed.is_err() || *drive.stop_rx.borrow() {
                    break;
                }
            }
            input = drive.input_rx.recv(), if drive.allow_input => {
                match input {
                    Some(event) => send_input_event(&drive.live.conn, &event).await.map_err(cli)?,
                    None => break,
                }
            }
            body = recv_uni(&drive.live.conn) => {
                let picture = take_uni(&mut drive.live, body.map_err(cli)?)?;
                if let Some(picture) = picture {
                    note_picture(drive, &picture, &mut wrote_ppm, &mut sent_script, &mut report).await?;
                }
            }
            clip = recv_clipboard_text(&drive.live.conn), if drive.allow_clip => {
                apply_clip(&mut clips, clip.map_err(cli)?)?;
            }
            _ = &mut pause, if drive.live.mic.is_some() || drive.allow_clip => {
                flush_mic(&drive.live).await?;
                if drive.allow_clip {
                    push_local_clip(&drive.live.conn, &mut clips).await?;
                }
            }
        }
    }
    Ok(())
}

async fn note_picture(
    drive: &mut WindowDrive,
    picture: &Picture,
    wrote_ppm: &mut bool,
    sent_script: &mut bool,
    report: &mut ReportClock,
) -> Result<(), CliError> {
    if let Some(path) = drive.ppm.as_ref() {
        if !*wrote_ppm {
            write_ppm(path, picture)?;
            *wrote_ppm = true;
        }
    }
    if let Ok(mut slot) = drive.frames.lock() {
        *slot = Some(picture.clone());
    }
    if !*sent_script {
        send_scripted(&drive.live, drive.pointer, drive.key.as_deref()).await?;
        *sent_script = true;
    }
    report.send(&drive.live.conn, 0).await
}

async fn run_remote(live: &mut LiveView, args: &ViewArgs) -> Result<(), CliError> {
    let mut seen_audio = false;
    if args.ppm.is_some() {
        let picture = next_picture(live, &mut seen_audio).await?;
        if let Some(path) = args.ppm.as_ref() {
            write_ppm(path, &picture)?;
        }
        ReportClock::default().send(&live.conn, 1).await?;
        send_scripted(live, args.pointer, args.key.as_deref()).await?;
    }
    if let Some(path) = args.put.as_ref() {
        send_while_reading(live, path, &mut seen_audio).await?;
    }
    if (args.audio || args.speaker) && !seen_audio {
        wait_audio(live).await?;
    }
    flush_mic(live).await?;
    Ok(())
}

async fn next_picture(live: &mut LiveView, seen_audio: &mut bool) -> Result<Picture, CliError> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            return Err(CliError::new("timed out waiting for a frame"));
        }
        flush_mic(live).await?;
        let slice = if live.mic.is_some() {
            remain.min(Duration::from_millis(20))
        } else {
            remain
        };
        let body = tokio::time::timeout(slice, recv_uni(&live.conn))
            .await
            .map_err(|_| CliError::new("timed out waiting for a frame"))?
            .map_err(cli)?;
        let was_audio = matches!(body, UniBody::Audio(_));
        if let Some(picture) = take_uni(live, body)? {
            return Ok(picture);
        }
        if was_audio {
            *seen_audio = true;
        }
    }
}

async fn send_while_reading(
    live: &mut LiveView,
    path: &std::path::Path,
    seen_audio: &mut bool,
) -> Result<(), CliError> {
    let chunks = crate::files::load_chunks(path)?;
    let mut index = 0;
    let conn = &live.conn;
    let decoder = &mut live.decoder;
    let canvas = &mut live.canvas;
    let codec = live.codec;
    while index < chunks.len() {
        tokio::select! {
            biased;
            result = cbdesk_net::send_file_chunk(conn, &chunks[index]) => {
                result.map_err(cli)?;
                index += 1;
            }
            body = recv_uni(conn) => {
                let body = body.map_err(cli)?;
                if matches!(body, UniBody::Audio(_)) {
                    *seen_audio = true;
                }
                if let UniBody::Video(header, payload) = body {
                    let expected = codec.ok_or_else(|| {
                        CliError::new("host sent video that was not negotiated")
                    })?;
                    decode_video(canvas, decoder, expected, &header, &payload)?;
                }
            }
        }
    }
    wait_uni(conn, |body| matches!(body, UniBody::FileAck), "file ack").await
}

async fn wait_audio(live: &mut LiveView) -> Result<(), CliError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            return Err(CliError::new("timed out waiting for audio"));
        }
        flush_mic(live).await?;
        let slice = if live.mic.is_some() {
            remain.min(Duration::from_millis(20))
        } else {
            remain
        };
        let body = tokio::time::timeout(slice, recv_uni(&live.conn))
            .await
            .map_err(|_| CliError::new("timed out waiting for audio"))?
            .map_err(cli)?;
        let was_audio = matches!(body, UniBody::Audio(_));
        let _ = take_uni(live, body)?;
        if was_audio {
            return Ok(());
        }
    }
}

fn take_uni(live: &mut LiveView, body: UniBody) -> Result<Option<Picture>, CliError> {
    match body {
        UniBody::Video(header, payload) => {
            let codec = live
                .codec
                .ok_or_else(|| CliError::new("host sent video that was not negotiated"))?;
            Ok(Some(decode_video(
                &mut live.canvas,
                &mut live.decoder,
                codec,
                &header,
                &payload,
            )?))
        }
        UniBody::Audio(packet) => {
            println!("audio {}", packet.len());
            play_incoming(live, &packet)?;
            Ok(None)
        }
        UniBody::File(_) | UniBody::Feedback(_) | UniBody::InputAck | UniBody::FileAck => Ok(None),
    }
}

fn decode_video(
    canvas: &mut Option<Picture>,
    decoder: &mut Vp8StreamDecoder,
    expected: Codec,
    header: &VideoFrameHeader,
    payload: &[u8],
) -> Result<Picture, CliError> {
    if header.codec != expected {
        return Err(CliError::new("host sent a codec that was not negotiated"));
    }
    match header.codec {
        Codec::Vp8 => composite_vp8(
            canvas,
            decoder,
            PlacedFrame {
                unchanged: header.unchanged,
                damage_x: header.damage_x,
                damage_y: header.damage_y,
                damage_width: header.damage_width,
                damage_height: header.damage_height,
                payload,
            },
        )
        .map_err(cli),
        Codec::Av1 => {
            let frame = decode_luma(payload).map_err(cli)?;
            let picture = Picture::from_luma(&frame).map_err(cli)?;
            *canvas = Some(picture.clone());
            Ok(picture)
        }
        _ => Err(CliError::new("host sent a codec this build cannot decode")),
    }
}

#[derive(Default)]
struct ReportClock {
    last: Option<Instant>,
}

impl ReportClock {
    async fn send(&mut self, conn: &QuicConn, frame_no: u64) -> Result<(), CliError> {
        let due = self
            .last
            .map(|last| last.elapsed() >= Duration::from_millis(100))
            .unwrap_or(true);
        if !due {
            return Ok(());
        }
        let delay = self
            .last
            .map(|last| elapsed_u32(last.elapsed()))
            .unwrap_or(0);
        self.last = Some(Instant::now());
        let feedback = Feedback {
            recv_kbps: 0,
            loss: 0.0,
            frame_delay_ms: delay,
            last_frame_no: frame_no,
            unknown: UnknownFields::new(),
        };
        send_feedback(conn, &feedback).await.map_err(cli)
    }
}

async fn send_scripted(
    live: &LiveView,
    pointer: Option<(i32, i32)>,
    key: Option<&str>,
) -> Result<(), CliError> {
    if pointer.is_none() && key.is_none() {
        return Ok(());
    }
    if !live.grant.granted.perms.contains(&Permission::Input) {
        return Err(CliError::new("host did not grant input"));
    }
    if let Some((x, y)) = pointer {
        let event = pointer_event(x, y);
        send_input_event(&live.conn, &event).await.map_err(cli)?;
    }
    if let Some(name) = key {
        let event = window::scripted_key(name)?;
        send_input_event(&live.conn, &event).await.map_err(cli)?;
    }
    let acks = usize::from(pointer.is_some()) + usize::from(key.is_some());
    let mut got = 0;
    wait_uni(
        &live.conn,
        |body| {
            if matches!(body, UniBody::InputAck) {
                got += 1;
            }
            got >= acks
        },
        "input ack",
    )
    .await
}

async fn wait_uni(
    conn: &QuicConn,
    mut done: impl FnMut(&UniBody) -> bool,
    label: &str,
) -> Result<(), CliError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            return Err(CliError::new(format!("timed out waiting for {label}")));
        }
        let body = tokio::time::timeout(remain, recv_uni(conn))
            .await
            .map_err(|_| CliError::new(format!("timed out waiting for {label}")))?
            .map_err(cli)?;
        if done(&body) {
            return Ok(());
        }
    }
}

fn pointer_event(x: i32, y: i32) -> InputEvent {
    use cbdesk_proto::{InputBody, PointerEvent};
    InputEvent {
        body: InputBody::Pointer(PointerEvent {
            x,
            y,
            buttons: 0,
            wheel: 0,
            unknown: UnknownFields::new(),
        }),
        unknown: UnknownFields::new(),
    }
}

async fn connect_view(args: &ViewArgs) -> Result<LiveView, CliError> {
    let devices = open_view_devices(args)?;
    let (endpoint, identity) = bind_viewer(args).await?;
    let conn = if args.browse {
        let (peer, source) = browse_peer(&identity).await?;
        let addrs = dial_candidates(source);
        connect_candidates(&endpoint, peer.as_bytes(), &addrs)
            .await
            .map_err(cli)?
    } else {
        dial_ticket(&endpoint, args).await?
    };
    let grant = match open_controller(&conn, &hello(args)).await.map_err(cli)? {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => {
            return Err(CliError::new(format!("host closed: {:?}", close.code)));
        }
    };
    let needs_view = args.ppm.is_some() || args.window;
    if needs_view && !grant.granted.perms.contains(&Permission::View) {
        return Err(CliError::new("host did not grant view"));
    }
    if args.put.is_some() && !grant.granted.perms.contains(&Permission::FileWrite) {
        return Err(CliError::new("host did not grant file_write"));
    }
    if (args.audio || args.mic || args.speaker) && !grant.granted.perms.contains(&Permission::Audio)
    {
        return Err(CliError::new("host did not grant audio"));
    }
    let codec = select_video_codec(&local_capabilities(), &grant.host_caps);
    if needs_view && codec.is_none() {
        return Err(CliError::new("no shared video codec"));
    }
    Ok(LiveView {
        _endpoint: endpoint,
        conn,
        grant,
        codec,
        decoder: Vp8StreamDecoder::new(),
        canvas: None,
        _identity: identity,
        mic: devices.mic,
        speaker: devices.speaker,
        playback: devices.playback,
    })
}

struct ViewDevices {
    mic: Option<CapturePump>,
    speaker: Option<Speaker>,
    playback: Option<PacketCodec>,
}

fn open_view_devices(args: &ViewArgs) -> Result<ViewDevices, CliError> {
    let mic = if args.mic {
        Some(CapturePump::start().map_err(|err| CliError::new(err.to_string()))?)
    } else {
        None
    };
    let speaker = if args.speaker {
        Some(Speaker::open_default().map_err(|err| CliError::new(err.to_string()))?)
    } else {
        None
    };
    let playback = if speaker.is_some() {
        Some(PacketCodec::new().map_err(|err| CliError::new(err.to_string()))?)
    } else {
        None
    };
    Ok(ViewDevices {
        mic,
        speaker,
        playback,
    })
}

async fn flush_mic(live: &LiveView) -> Result<(), CliError> {
    // Own the packets before the await. `pop` locks the queue for one try,
    // and the send must not hold that lock.
    let packets = match &live.mic {
        Some(mic) => {
            let mut packets = Vec::new();
            while let Some(packet) = mic.pop() {
                packets.push(packet);
            }
            packets
        }
        None => return Ok(()),
    };
    for packet in packets {
        send_audio_packet(&live.conn, &packet).await.map_err(cli)?;
    }
    Ok(())
}

fn play_incoming(live: &mut LiveView, packet: &[u8]) -> Result<(), CliError> {
    let (Some(playback), Some(speaker)) = (live.playback.as_mut(), live.speaker.as_ref()) else {
        return Ok(());
    };
    let pcm = playback
        .decode_packet(packet)
        .map_err(|err| CliError::new(err.to_string()))?;
    speaker
        .play(&pcm)
        .map_err(|err| CliError::new(err.to_string()))
}

async fn dial_ticket(endpoint: &QuicEndpoint, args: &ViewArgs) -> Result<QuicConn, CliError> {
    let ticket_hex = args
        .ticket_hex
        .as_deref()
        .ok_or_else(|| CliError::new("view needs --ticket or --browse"))?;
    let bytes = hex::decode(ticket_hex.trim())?;
    let ticket = cbdesk_proto::InviteTicket::from_bytes(&bytes).map_err(cli)?;
    let mut replay = ReplayCache::new();
    dial_verified_ticket(endpoint, &ticket, unix_now(), &mut replay)
        .await
        .map_err(cli)
}

fn apply_clip(clips: &mut ClipSync, text: ClipboardText) -> Result<(), CliError> {
    if !clips.note_remote(&text.text) || clips.os_broken() {
        return Ok(());
    }
    if let Err(err) = OsClipboard::set(&text.text) {
        clips.mark_os_broken();
        eprintln!("cbdesk: clipboard is unavailable: {err}");
    }
    Ok(())
}

async fn push_local_clip(conn: &QuicConn, clips: &mut ClipSync) -> Result<(), CliError> {
    if clips.os_broken() {
        return Ok(());
    }
    let text = match OsClipboard::get() {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(()),
        Err(err) => {
            clips.mark_os_broken();
            eprintln!("cbdesk: clipboard is unavailable: {err}");
            return Ok(());
        }
    };
    let Some(generation) = clips.local_change(&text) else {
        return Ok(());
    };
    let message = ClipboardText {
        text,
        generation,
        unknown: UnknownFields::new(),
    };
    send_clipboard_text(conn, &message).await.map_err(cli)
}

async fn bind_viewer(args: &ViewArgs) -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = open_identity()?;
    let mut seed = identity.signing_seed();
    let addr = "127.0.0.1:0"
        .parse()
        .map_err(|_| CliError::new("viewer bind address is invalid"))?;
    let config = EndpointConfig {
        relays: args.relays.clone(),
        ..EndpointConfig::default()
    };
    let endpoint = QuicEndpoint::bind(&config, addr, &seed)
        .await
        .map_err(cli)?;
    seed.zeroize();
    Ok((endpoint, identity))
}

fn hello(args: &ViewArgs) -> Hello {
    let mut wanted = Vec::new();
    if args.ppm.is_some() || args.window {
        wanted.push(Permission::View);
    }
    if args.pointer.is_some() || args.key.is_some() || args.window {
        wanted.push(Permission::Input);
    }
    if args.window {
        wanted.push(Permission::Clipboard);
    }
    if args.put.is_some() {
        wanted.push(Permission::FileWrite);
    }
    if args.audio || args.mic || args.speaker {
        wanted.push(Permission::Audio);
    }
    Hello {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        app_version: "0.0.0".to_string(),
        platform: local_platform(),
        caps: local_capabilities().to_vec(),
        wanted_perms: wanted,
        auth_offer: vec![AuthMethod::Interactive],
        unknown: UnknownFields::new(),
        ..Hello::default()
    }
}

fn local_platform() -> Platform {
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        Platform::Macos
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Platform::Unspecified
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn elapsed_u32(duration: Duration) -> u32 {
    u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
}

fn cli(err: impl ToString) -> CliError {
    CliError::new(err.to_string())
}

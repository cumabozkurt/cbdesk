// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host one controller until that controller closes the session.
//!
//! `--strips 1` (the default) keeps one damage-encoder frame. A larger
//! count, only when the negotiated codec is VP8, sends each horizontal
//! band as its own keyframe and names the full canvas on the header.
//! Other codecs ignore the flag. The desk host does not take it.

use std::io::{self, Write};
use std::net::IpAddr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cbdesk_audio::{CapturePump, PacketCodec, Speaker, ToneCodec};
use cbdesk_capture::{
    capture_picture, capture_picture_on, crop_even, Picture, SyntheticCapturer, BARCODE_WIDTH,
};
use cbdesk_codec::{
    encode_luma, encode_strips, scale_profile, BitrateControl, CodecError, DamageEncoder,
    DamagePacket, EncodeQuality, FrameProfile, Pace, StripSet,
};
use cbdesk_crypto::Identity;
use cbdesk_discovery::sign_ticket;
use cbdesk_input::{inject_os, Injector, InputLog};
use cbdesk_net::{admit_invited, ConsentAnswer, EndpointConfig, HostLink, PeerEvent, QuicEndpoint};
use cbdesk_proto::{
    local_capabilities, select_video_codec, ClipboardText, Codec, InputBody, InputEvent,
    InvitePurpose, Permission, ProtoMessage, UnknownFields, VideoFrameHeader,
};
use cbdesk_session::{
    ApprovalMode, HostSession, PeerFacts, RateLimiter, SecurityConfig, CONSENT_DELAY_MS,
};
use zeroize::Zeroize;

use crate::args::HostArgs;
use crate::clipboard::{outgoing_clipboard, ClipSync, OsClipboard};
use crate::error::CliError;
use crate::files::{Stored, Uploads};
use crate::hex;
use crate::identity::open_identity;
use crate::lan::start_announce;

const FRAME_INTERVAL: Duration = Duration::from_millis(33);

struct Stream {
    encoder: DamageEncoder,
    rate: BitrateControl,
    skip_next: bool,
    last_hash: Option<[u8; 32]>,
    sent: u32,
    next_at: Instant,
    uploads: Option<Uploads>,
    tone: Option<ToneCodec>,
    mic: Option<CapturePump>,
    speaker: Option<Speaker>,
    playback: Option<PacketCodec>,
    profile: FrameProfile,
    /// VP8 band count. One stays on [`DamageEncoder`].
    strips: u32,
    /// Next `VideoFrameHeader::frame_no`. One picture may use several.
    next_frame: u64,
}

struct HostDevices {
    mic: Option<CapturePump>,
    speaker: Option<Speaker>,
    playback: Option<PacketCodec>,
}

/// Bind, print a ticket, and stream pictures until the viewer closes.
pub async fn serve(args: &HostArgs) -> Result<(), CliError> {
    let devices = open_host_devices(args)?;
    let (endpoint, identity) = bind_host(args).await?;
    if args.lan {
        start_announce(&endpoint, &identity).await?;
    }
    let ticket = sign_locator(&identity, &endpoint, args)?;
    println!("ticket {}", hex::encode(&ticket));
    io::stdout().flush()?;
    let conn = tokio::time::timeout(Duration::from_secs(45), endpoint.accept())
        .await
        .map_err(|_| CliError::new("timed out waiting for a controller"))?
        .map_err(|err| CliError::new(err.to_string()))?;
    let source = endpoint
        .local_addr()
        .map_err(|err| CliError::new(err.to_string()))?
        .ip();
    let mut link = admit(conn, source, args).await?;
    let codec = select_video_codec(&local_capabilities(), link.peer_caps())
        .ok_or_else(|| CliError::new("no shared video codec"))?;
    session_loop(&mut link, args, codec, devices).await
}

fn open_host_devices(args: &HostArgs) -> Result<HostDevices, CliError> {
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
    Ok(HostDevices {
        mic,
        speaker,
        playback,
    })
}

async fn bind_host(args: &HostArgs) -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = open_identity()?;
    let mut seed = identity.signing_seed();
    let config = EndpointConfig {
        relays: args.relays.clone(),
        ..EndpointConfig::default()
    };
    let endpoint = QuicEndpoint::bind(&config, args.bind, &seed)
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    seed.zeroize();
    Ok((endpoint, identity))
}

fn sign_locator(
    identity: &Identity,
    endpoint: &QuicEndpoint,
    args: &HostArgs,
) -> Result<Vec<u8>, CliError> {
    let addr = endpoint
        .local_addr()
        .map_err(|err| CliError::new(err.to_string()))?;
    let expires = unix_now().saturating_add(10 * 60);
    let ticket = sign_ticket(
        cbdesk_proto::InviteTicket {
            direct_addrs: vec![addr.to_string()],
            relay_urls: args.relays.clone(),
            expires_at: expires,
            purpose: InvitePurpose::Session,
            ..cbdesk_proto::InviteTicket::default()
        },
        identity,
    )
    .map_err(|err| CliError::new(err.to_string()))?;
    ticket
        .to_bytes()
        .map_err(|err| CliError::new(err.to_string()))
}

async fn admit(
    conn: cbdesk_net::QuicConn,
    source: IpAddr,
    args: &HostArgs,
) -> Result<HostLink, CliError> {
    let origin = Instant::now();
    let now_ms = millis_since(origin);
    let perms = args.approve.clone();
    let high_risk = args.allow_high_risk;
    let answer = async move { consent_answer(origin, perms, high_risk).await };
    let session = HostSession::new(SecurityConfig {
        approval: ApprovalMode::Always,
        unattended_enabled: false,
        require_2fa: false,
        quick_support: false,
    })
    .map_err(|err| CliError::new(err.to_string()))?;
    admit_invited(
        conn,
        session,
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        source,
        now_ms,
        answer,
    )
    .await
    .map_err(|failed| CliError::new(failed.error.to_string()))
}

async fn consent_answer(origin: Instant, perms: Vec<Permission>, high_risk: bool) -> ConsentAnswer {
    if perms.is_empty() {
        return ConsentAnswer::Reject {
            now_ms: millis_since(origin),
        };
    }
    tokio::time::sleep(Duration::from_millis(CONSENT_DELAY_MS)).await;
    ConsentAnswer::Approve {
        now_ms: millis_since(origin),
        perms,
        explicit_high_risk: high_risk,
    }
}

async fn session_loop(
    link: &mut HostLink,
    args: &HostArgs,
    codec: Codec,
    devices: HostDevices,
) -> Result<(), CliError> {
    let mut stream = Stream::prepare(args, devices)?;
    let mut clips = ClipSync::new();
    let clip_ok = link.authorize(Permission::Clipboard).is_ok();
    let view_ok = link.authorize(Permission::View).is_ok();
    let audio_ok = (args.tone || args.mic) && link.authorize(Permission::Audio).is_ok();
    loop {
        stream.pump(link, args, codec, view_ok, audio_ok).await?;
        if !wait_peer(
            link,
            args,
            &mut stream,
            &mut clips,
            clip_ok,
            view_ok,
            audio_ok,
        )
        .await?
        {
            break;
        }
    }
    Ok(())
}

impl Stream {
    fn prepare(args: &HostArgs, devices: HostDevices) -> Result<Self, CliError> {
        let uploads = match &args.files_root {
            Some(root) => Some(Uploads::create(root)?),
            None => None,
        };
        let tone = if args.tone {
            Some(ToneCodec::new().map_err(|err| CliError::new(err.to_string()))?)
        } else {
            None
        };
        Ok(Self {
            encoder: DamageEncoder::new(),
            rate: BitrateControl::live(),
            skip_next: false,
            last_hash: None,
            sent: 0,
            next_at: Instant::now(),
            uploads,
            tone,
            mic: devices.mic,
            speaker: devices.speaker,
            playback: devices.playback,
            profile: args.profile,
            strips: args.strips,
            next_frame: 1,
        })
    }

    async fn pump(
        &mut self,
        link: &mut HostLink,
        args: &HostArgs,
        codec: Codec,
        view_ok: bool,
        audio_ok: bool,
    ) -> Result<(), CliError> {
        if Instant::now() < self.next_at {
            return Ok(());
        }
        let send_video = view_ok && wants_frame(self.sent, args.frames);
        let send_audio = audio_ok && (self.tone.is_some() || self.mic.is_some());
        if !send_video && !send_audio {
            return Ok(());
        }
        if self.skip_next {
            self.skip_next = false;
            self.arm_interval();
            return Ok(());
        }
        if send_video {
            self.send_picture(link, args.screen, args.monitor, codec)
                .await?;
        } else {
            self.arm_interval();
        }
        if send_audio {
            self.send_tone(link).await?;
        }
        Ok(())
    }

    async fn send_picture(
        &mut self,
        link: &mut HostLink,
        screen: bool,
        monitor: Option<u32>,
        codec: Codec,
    ) -> Result<(), CliError> {
        let capture_no = u64::from(self.sent.saturating_add(1));
        let captured = capture(screen, capture_no, monitor)?;
        let digest = picture_hash(&captured);
        if self.last_hash == Some(digest) {
            self.arm_interval();
            return Ok(());
        }
        let picture =
            scale_profile(&captured, self.profile).map_err(|err| CliError::new(err.to_string()))?;
        let qindex = if screen && self.profile != FrameProfile::Full {
            self.profile.qindex()
        } else if screen {
            self.rate.qindex()
        } else {
            0
        };
        if self.strips > 1 && codec == Codec::Vp8 {
            return self
                .send_strips(link, picture, qindex, digest, screen)
                .await;
        }
        let started = Instant::now();
        let frame = self.encode(picture, codec, qindex, screen).await?;
        self.note_encode(screen, elapsed_ms(started));
        if frame.unchanged {
            self.last_hash = Some(digest);
            self.arm_interval();
            return Ok(());
        }
        let frame_no = self.take_frame_no();
        let header = video_header(codec, frame_no, &frame);
        link.send_view_frame(&header, &frame.bytes)
            .await
            .map_err(|err| CliError::new(err.to_string()))?;
        self.sent = self.sent.saturating_add(1);
        self.last_hash = Some(digest);
        self.arm_interval();
        Ok(())
    }

    async fn send_strips(
        &mut self,
        link: &mut HostLink,
        picture: Picture,
        qindex: u8,
        digest: [u8; 32],
        screen: bool,
    ) -> Result<(), CliError> {
        let strips = self.strips;
        let started = Instant::now();
        let encoded =
            tokio::task::spawn_blocking(move || encode_strips(&picture, qindex, strips, true))
                .await
                .map_err(|err| CliError::new(err.to_string()))?
                .map_err(|err| CliError::new(err.to_string()))?;
        self.note_encode(screen, elapsed_ms(started));
        if encoded.packets.is_empty() {
            return Err(CliError::new("strip encode produced no bands"));
        }
        let (frames, next_frame) = frames_for_strips(&encoded, self.next_frame);
        for frame in &frames {
            link.send_view_frame(&frame.header, &frame.bytes)
                .await
                .map_err(|err| CliError::new(err.to_string()))?;
        }
        self.next_frame = next_frame;
        self.sent = self.sent.saturating_add(1);
        self.last_hash = Some(digest);
        self.arm_interval();
        Ok(())
    }

    fn take_frame_no(&mut self) -> u64 {
        let no = self.next_frame;
        self.next_frame = self.next_frame.saturating_add(1);
        no
    }

    async fn encode(
        &mut self,
        picture: Picture,
        codec: Codec,
        qindex: u8,
        screen: bool,
    ) -> Result<OutgoingFrame, CliError> {
        match codec {
            Codec::Vp8 => encode_vp8_stream(&mut self.encoder, picture, qindex).await,
            Codec::Av1 => encode_av1(picture, screen)
                .await
                .map(|bytes| OutgoingFrame {
                    bytes,
                    keyframe: true,
                    unchanged: false,
                    damage_x: 0,
                    damage_y: 0,
                    damage_width: 0,
                    damage_height: 0,
                }),
            _ => Err(CliError::new("codec is not available")),
        }
    }

    fn note_encode(&mut self, screen: bool, elapsed: u32) {
        if screen && self.rate.observe_encode(elapsed) == Pace::SkipNext {
            self.skip_next = true;
        }
    }

    async fn send_tone(&mut self, link: &mut HostLink) -> Result<(), CliError> {
        if let Some(tone) = self.tone.as_mut() {
            let packet = tone
                .encode_frame()
                .map_err(|err| CliError::new(err.to_string()))?;
            return link
                .send_audio(&packet)
                .await
                .map_err(|err| CliError::new(err.to_string()));
        }
        let packet = self.mic.as_ref().and_then(CapturePump::pop);
        if let Some(packet) = packet {
            link.send_audio(&packet)
                .await
                .map_err(|err| CliError::new(err.to_string()))?;
        }
        Ok(())
    }

    fn play_incoming(&mut self, packet: &[u8]) -> Result<(), CliError> {
        let (Some(playback), Some(speaker)) = (self.playback.as_mut(), self.speaker.as_ref())
        else {
            return Ok(());
        };
        let pcm = playback
            .decode_packet(packet)
            .map_err(|err| CliError::new(err.to_string()))?;
        speaker
            .play(&pcm)
            .map_err(|err| CliError::new(err.to_string()))
    }

    fn arm_interval(&mut self) {
        self.next_at = Instant::now() + FRAME_INTERVAL;
    }

    fn take_file(&mut self, chunk: &cbdesk_proto::FileChunk) -> Result<Stored, CliError> {
        let uploads = self
            .uploads
            .as_mut()
            .ok_or_else(|| CliError::new("file chunk arrived without --files-root"))?;
        uploads.push(chunk)
    }
}

async fn wait_peer(
    link: &mut HostLink,
    args: &HostArgs,
    stream: &mut Stream,
    clips: &mut ClipSync,
    clip_ok: bool,
    view_ok: bool,
    audio_ok: bool,
) -> Result<bool, CliError> {
    let pacing = pacing_due(stream, args, view_ok, audio_ok);
    let waiting = pacing || (args.screen && clip_ok);
    let pause = tokio::time::sleep(if pacing {
        stream.next_at.saturating_duration_since(Instant::now())
    } else {
        Duration::from_millis(400)
    });
    tokio::pin!(pause);
    tokio::select! {
        biased;
        event = link.next_peer_event() => {
            match event {
                Ok(event) => apply_event(link, args, stream, clips, event).await,
                Err(err) => Err(CliError::new(err.to_string())),
            }
        }
        _ = &mut pause, if waiting => {
            if args.screen && clip_ok {
                push_local_clipboard(link, clips).await?;
            }
            Ok(true)
        }
    }
}

fn pacing_due(stream: &Stream, args: &HostArgs, view_ok: bool, audio_ok: bool) -> bool {
    let video = view_ok && wants_frame(stream.sent, args.frames);
    video || audio_ok
}

async fn apply_event(
    link: &HostLink,
    args: &HostArgs,
    stream: &mut Stream,
    clips: &mut ClipSync,
    event: PeerEvent,
) -> Result<bool, CliError> {
    match event {
        PeerEvent::Ended | PeerEvent::Control(_) => Ok(false),
        PeerEvent::Input(event) => {
            record_input(&event, args.screen)?;
            Ok(true)
        }
        PeerEvent::Clipboard(text) => {
            apply_remote_clipboard(clips, &text.text, args.screen)?;
            Ok(true)
        }
        PeerEvent::File(chunk) => {
            let stored = note_file(stream, &chunk)?;
            if stored == Stored::Done {
                link.ack_file()
                    .await
                    .map_err(|err| CliError::new(err.to_string()))?;
            }
            Ok(true)
        }
        PeerEvent::Audio(packet) => {
            println!("audio {}", packet.len());
            io::stdout().flush()?;
            stream.play_incoming(&packet)?;
            Ok(true)
        }
        PeerEvent::Feedback(feedback) => {
            if args.screen {
                stream
                    .rate
                    .observe_feedback(feedback.loss, feedback.frame_delay_ms);
            }
            Ok(true)
        }
    }
}

fn note_file(stream: &mut Stream, chunk: &cbdesk_proto::FileChunk) -> Result<Stored, CliError> {
    let stored = stream.take_file(chunk)?;
    match stored {
        Stored::Part => println!("file {} {}", chunk.offset, chunk.path),
        Stored::Done => println!("file complete {}", chunk.path),
    }
    io::stdout().flush()?;
    Ok(stored)
}

fn wants_frame(sent: u32, limit: u32) -> bool {
    limit == 0 || sent < limit
}

struct OutgoingFrame {
    bytes: Vec<u8>,
    keyframe: bool,
    unchanged: bool,
    damage_x: u32,
    damage_y: u32,
    damage_width: u32,
    damage_height: u32,
}

fn video_header(codec: Codec, frame_no: u64, frame: &OutgoingFrame) -> VideoFrameHeader {
    VideoFrameHeader {
        display_id: 1,
        codec,
        frame_no,
        capture_ts_us: 1_000,
        keyframe: frame.keyframe,
        damage_x: frame.damage_x,
        damage_y: frame.damage_y,
        damage_width: frame.damage_width,
        damage_height: frame.damage_height,
        unchanged: false,
        ..VideoFrameHeader::default()
    }
}

struct WireFrame {
    header: VideoFrameHeader,
    bytes: Vec<u8>,
}

/// One band stays a full frame so the default header does not grow.
/// Two or more bands name the canvas and the damage rect of each band.
fn frames_for_strips(set: &StripSet, mut frame_no: u64) -> (Vec<WireFrame>, u64) {
    if set.packets.len() <= 1 {
        let bytes = set
            .packets
            .first()
            .map(|packet| packet.bytes.clone())
            .unwrap_or_default();
        let header = video_header(
            Codec::Vp8,
            frame_no,
            &OutgoingFrame {
                bytes: Vec::new(),
                keyframe: true,
                unchanged: false,
                damage_x: 0,
                damage_y: 0,
                damage_width: 0,
                damage_height: 0,
            },
        );
        frame_no = frame_no.saturating_add(1);
        return (vec![WireFrame { header, bytes }], frame_no);
    }
    let count = u32::try_from(set.packets.len()).unwrap_or(u32::MAX);
    let mut frames = Vec::with_capacity(set.packets.len());
    for (index, packet) in set.packets.iter().enumerate() {
        frames.push(WireFrame {
            header: VideoFrameHeader {
                display_id: 1,
                codec: Codec::Vp8,
                frame_no,
                capture_ts_us: 1_000,
                keyframe: true,
                damage_y: packet.origin_y,
                damage_width: packet.width,
                damage_height: packet.height,
                canvas_width: set.width,
                canvas_height: set.height,
                strip_index: u32::try_from(index).unwrap_or(0),
                strip_count: count,
                ..VideoFrameHeader::default()
            },
            bytes: packet.bytes.clone(),
        });
        frame_no = frame_no.saturating_add(1);
    }
    (frames, frame_no)
}

fn place(packet: DamagePacket) -> OutgoingFrame {
    let region = packet.region;
    OutgoingFrame {
        bytes: packet.bytes,
        keyframe: packet.keyframe && !region,
        unchanged: packet.unchanged,
        damage_x: if region { packet.origin_x } else { 0 },
        damage_y: if region { packet.origin_y } else { 0 },
        damage_width: if region { packet.width } else { 0 },
        damage_height: if region { packet.height } else { 0 },
    }
}

async fn encode_vp8_stream(
    encoder: &mut DamageEncoder,
    picture: Picture,
    qindex: u8,
) -> Result<OutgoingFrame, CliError> {
    let taken = std::mem::take(encoder);
    let encoded = tokio::task::spawn_blocking(move || {
        let mut taken = taken;
        let result = taken.encode(&picture, qindex);
        (taken, result)
    })
    .await
    .map_err(|err| CliError::new(err.to_string()))?;
    *encoder = encoded.0;
    encoded
        .1
        .map(place)
        .map_err(|err| CliError::new(err.to_string()))
}

async fn encode_av1(picture: Picture, screen: bool) -> Result<Vec<u8>, CliError> {
    let frame = picture
        .to_luma()
        .map_err(|err| CliError::new(err.to_string()))?;
    let quality = if screen {
        EncodeQuality::Fast
    } else {
        EncodeQuality::Lossless
    };
    tokio::task::spawn_blocking(move || encode_frame(&frame, quality))
        .await
        .map_err(|err| CliError::new(err.to_string()))?
        .map_err(|err| CliError::new(err.to_string()))
}

fn encode_frame(
    frame: &cbdesk_capture::Frame,
    quality: EncodeQuality,
) -> Result<Vec<u8>, CodecError> {
    encode_luma(frame, quality)
}

fn capture(screen: bool, frame_no: u64, monitor: Option<u32>) -> Result<Picture, CliError> {
    if screen {
        let picture = match monitor {
            Some(index) => capture_picture_on(index),
            None => capture_picture(),
        }
        .map_err(|err| CliError::new(err.to_string()))?;
        return crop_even(picture).map_err(|err| CliError::new(err.to_string()));
    }
    if monitor.is_some() {
        return Err(CliError::new("--monitor is for --screen"));
    }
    let frame = SyntheticCapturer::new(BARCODE_WIDTH, 16)
        .capture(frame_no, 1_000)
        .map_err(|err| CliError::new(err.to_string()))?;
    Picture::from_luma(&frame).map_err(|err| CliError::new(err.to_string()))
}

fn picture_hash(picture: &Picture) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&picture.y);
    hasher.update(&picture.u);
    hasher.update(&picture.v);
    *hasher.finalize().as_bytes()
}

fn record_input(event: &InputEvent, screen: bool) -> Result<(), CliError> {
    if screen {
        inject_os(event).map_err(|err| CliError::new(err.to_string()))?;
    } else {
        let mut log = InputLog::new();
        log.inject(event)
            .map_err(|err| CliError::new(err.to_string()))?;
    }
    print_input(event)
}

fn print_input(event: &InputEvent) -> Result<(), CliError> {
    match &event.body {
        InputBody::Pointer(pointer) => println!("input pointer {} {}", pointer.x, pointer.y),
        InputBody::Key(key) => println!("input key {} {}", key.keycode, u8::from(key.down)),
    }
    io::stdout().flush()?;
    Ok(())
}

fn apply_remote_clipboard(clips: &mut ClipSync, text: &str, screen: bool) -> Result<(), CliError> {
    let text = match outgoing_clipboard(text) {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(()),
        Err(err) => {
            eprintln!("cbdesk: clipboard was not applied: {err}");
            return Ok(());
        }
    };
    if !clips.note_remote(&text) {
        return Ok(());
    }
    if screen && !clips.os_broken() {
        if let Err(err) = OsClipboard::set(&text) {
            clips.mark_os_broken();
            eprintln!("cbdesk: clipboard is unavailable: {err}");
        }
    }
    println!("clipboard {}", text.len());
    io::stdout().flush()?;
    Ok(())
}

async fn push_local_clipboard(link: &mut HostLink, clips: &mut ClipSync) -> Result<(), CliError> {
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
    let text = match outgoing_clipboard(&text) {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(()),
        Err(err) => {
            eprintln!("cbdesk: clipboard was not sent: {err}");
            return Ok(());
        }
    };
    let message = ClipboardText {
        text,
        generation,
        unknown: UnknownFields::new(),
    };
    link.send_clipboard(&message)
        .await
        .map_err(|err| CliError::new(err.to_string()))
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

fn millis_since(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use cbdesk_codec::{StripPacket, StripSet};

    use super::*;

    fn band(origin_y: u32, height: u32, byte: u8) -> StripPacket {
        StripPacket {
            origin_y,
            width: 32,
            height,
            bytes: vec![byte],
        }
    }

    #[test]
    fn an_index_past_any_list_is_refused_and_synthetic_ignores_none() {
        assert!(capture(true, 1, Some(u32::MAX)).is_err());
        assert!(capture(false, 1, Some(1))
            .expect_err("synthetic")
            .to_string()
            .contains("--monitor is for --screen"));
        let barcode = capture(false, 1, None).expect("barcode");
        assert!(barcode.width >= 1 && barcode.height >= 1);
    }

    #[test]
    fn one_band_stays_a_full_frame_and_two_bands_name_the_canvas() {
        let single = StripSet {
            width: 32,
            height: 32,
            qindex: 0,
            parallel: false,
            wall_ms: 0,
            packets: vec![band(0, 32, 1)],
        };
        let (frames, next) = frames_for_strips(&single, 1);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].header.frame_no, 1);
        assert!(frames[0].header.keyframe);
        assert_eq!(frames[0].header.damage_width, 0);
        assert_eq!(frames[0].header.canvas_width, 0);
        assert_eq!(frames[0].header.strip_count, 0);
        assert_eq!(frames[0].bytes, vec![1]);
        assert_eq!(next, 2);

        let two = StripSet {
            width: 32,
            height: 32,
            qindex: 8,
            parallel: true,
            wall_ms: 1,
            packets: vec![band(0, 16, 1), band(16, 16, 2)],
        };
        let (frames, next) = frames_for_strips(&two, 3);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].header.frame_no, 3);
        assert_eq!(frames[1].header.frame_no, 4);
        assert_eq!(next, 5);
        assert_eq!(frames[0].header.damage_y, 0);
        assert_eq!(frames[1].header.damage_y, 16);
        assert_eq!(frames[0].header.damage_width, 32);
        assert_eq!(frames[0].header.damage_height, 16);
        assert_eq!(frames[0].header.canvas_width, 32);
        assert_eq!(frames[0].header.canvas_height, 32);
        assert_eq!(frames[0].header.strip_index, 0);
        assert_eq!(frames[1].header.strip_index, 1);
        assert_eq!(frames[0].header.strip_count, 2);
        assert!(frames[0].header.keyframe);
        assert_eq!(frames[1].bytes, vec![2]);
    }
}

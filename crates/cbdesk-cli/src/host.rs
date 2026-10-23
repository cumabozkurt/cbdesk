// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host one controller until that controller closes the session.

use std::io::{self, Write};
use std::net::IpAddr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cbdesk_capture::{capture_screen, fit_even, Frame, SyntheticCapturer, BARCODE_WIDTH};
use cbdesk_codec::{encode_luma, encode_vp8, CodecError, EncodeQuality};
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
use crate::clipboard::{ClipSync, OsClipboard};
use crate::error::CliError;
use crate::hex;
use crate::identity::open_identity;
use crate::lan::start_announce;

const FRAME_INTERVAL: Duration = Duration::from_millis(125);

/// Bind, print a ticket, and stream pictures until the viewer closes.
pub async fn serve(args: &HostArgs) -> Result<(), CliError> {
    let (endpoint, identity) = bind_host(args).await?;
    if args.lan {
        start_announce(&endpoint, &identity).await?;
    }
    let ticket = sign_locator(&identity, &endpoint)?;
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
    session_loop(&mut link, args, codec).await
}

async fn bind_host(args: &HostArgs) -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = open_identity()?;
    let mut seed = identity.signing_seed();
    let endpoint = QuicEndpoint::bind(&EndpointConfig::default(), args.bind, &seed)
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    seed.zeroize();
    Ok((endpoint, identity))
}

fn sign_locator(identity: &Identity, endpoint: &QuicEndpoint) -> Result<Vec<u8>, CliError> {
    let addr = endpoint
        .local_addr()
        .map_err(|err| CliError::new(err.to_string()))?;
    let expires = unix_now().saturating_add(10 * 60);
    let ticket = sign_ticket(
        cbdesk_proto::InviteTicket {
            direct_addrs: vec![addr.to_string()],
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

async fn session_loop(link: &mut HostLink, args: &HostArgs, codec: Codec) -> Result<(), CliError> {
    let mut sent = 0_u32;
    let mut next_frame = Instant::now();
    let mut clips = ClipSync::new();
    let clip_ok = link.authorize(Permission::Clipboard).is_ok();
    loop {
        if wants_frame(sent, args.frames) && Instant::now() >= next_frame {
            sent = sent.saturating_add(1);
            send_one(link, args.screen, codec, u64::from(sent)).await?;
            next_frame = Instant::now() + FRAME_INTERVAL;
        }
        if !drive_once(link, args, &mut clips, clip_ok, sent).await? {
            break;
        }
    }
    Ok(())
}

async fn drive_once(
    link: &mut HostLink,
    args: &HostArgs,
    clips: &mut ClipSync,
    clip_ok: bool,
    sent: u32,
) -> Result<bool, CliError> {
    let sending = wants_frame(sent, args.frames);
    let waiting = sending || (args.screen && clip_ok);
    let pause = tokio::time::sleep(if sending {
        FRAME_INTERVAL
    } else {
        Duration::from_millis(400)
    });
    tokio::pin!(pause);
    tokio::select! {
        biased;
        event = link.next_peer_event() => {
            match event {
                Ok(event) => apply_event(link, args, clips, event).await,
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

async fn apply_event(
    _link: &mut HostLink,
    args: &HostArgs,
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
    }
}

fn wants_frame(sent: u32, limit: u32) -> bool {
    limit == 0 || sent < limit
}

async fn send_one(
    link: &mut HostLink,
    screen: bool,
    codec: Codec,
    frame_no: u64,
) -> Result<(), CliError> {
    let (frame, quality) = one_frame(screen, frame_no)?;
    let payload = tokio::task::spawn_blocking(move || encode_frame(&frame, codec, quality))
        .await
        .map_err(|err| CliError::new(err.to_string()))?
        .map_err(|err| CliError::new(err.to_string()))?;
    let header = VideoFrameHeader {
        display_id: 1,
        codec,
        frame_no,
        capture_ts_us: 1_000,
        keyframe: true,
        unknown: UnknownFields::new(),
    };
    link.send_view_frame(&header, &payload)
        .await
        .map_err(|err| CliError::new(err.to_string()))
}

fn encode_frame(
    frame: &Frame,
    codec: Codec,
    quality: EncodeQuality,
) -> Result<Vec<u8>, CodecError> {
    match codec {
        Codec::Vp8 => encode_vp8(frame, quality),
        Codec::Av1 => encode_luma(frame, quality),
        _ => Err(CodecError::Encode("codec is not available".to_string())),
    }
}

fn one_frame(screen: bool, frame_no: u64) -> Result<(Frame, EncodeQuality), CliError> {
    if screen {
        let captured = capture_screen().map_err(|err| CliError::new(err.to_string()))?;
        let fitted = fit_even(&captured, 480, 270).map_err(|err| CliError::new(err.to_string()))?;
        return Ok((fitted, EncodeQuality::Fast));
    }
    let frame = SyntheticCapturer::new(BARCODE_WIDTH, 16)
        .capture(frame_no, 1_000)
        .map_err(|err| CliError::new(err.to_string()))?;
    Ok((frame, EncodeQuality::Lossless))
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
    if !clips.note_remote(text) {
        return Ok(());
    }
    if screen && !clips.os_broken() {
        if let Err(err) = OsClipboard::set(text) {
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
    let message = ClipboardText {
        text,
        generation,
        unknown: UnknownFields::new(),
    };
    link.send_clipboard(&message)
        .await
        .map_err(|err| CliError::new(err.to_string()))
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

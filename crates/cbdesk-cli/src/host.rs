// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host one controller. The ticket is printed before consent.

use std::io::{self, Write};
use std::net::IpAddr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cbdesk_capture::{capture_screen, fit_even, Frame, SyntheticCapturer, BARCODE_WIDTH};
use cbdesk_codec::{encode_luma, EncodeQuality};
use cbdesk_crypto::Identity;
use cbdesk_discovery::sign_ticket;
use cbdesk_input::{inject_os, Injector, InputLog};
use cbdesk_net::{admit_invited, ConsentAnswer, EndpointConfig, QuicEndpoint};
use cbdesk_proto::{
    Codec, InputBody, InputEvent, InvitePurpose, Permission, ProtoMessage, UnknownFields,
    VideoFrameHeader,
};
use cbdesk_session::{
    ApprovalMode, HostSession, PeerFacts, RateLimiter, SecurityConfig, CONSENT_DELAY_MS,
};
use zeroize::Zeroize;

use crate::args::HostArgs;
use crate::error::CliError;
use crate::hex;

/// Bind, print a ticket, and serve `args.frames` pictures after consent.
pub async fn serve(args: &HostArgs) -> Result<(), CliError> {
    let (endpoint, identity) = bind_host(args).await?;
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
    send_frames(&mut link, args).await?;
    if link.authorize(Permission::Input).is_ok() {
        take_input(&mut link, args.screen).await?;
    }
    // The viewer reads the frame, then finishes this stream. Leaving earlier
    // drops the frame before that read.
    let _ = tokio::time::timeout(Duration::from_secs(20), link.read_control()).await;
    Ok(())
}

async fn bind_host(args: &HostArgs) -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = Identity::generate().map_err(|err| CliError::new(err.to_string()))?;
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
) -> Result<cbdesk_net::HostLink, CliError> {
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

async fn send_frames(link: &mut cbdesk_net::HostLink, args: &HostArgs) -> Result<(), CliError> {
    for index in 1..=args.frames {
        let (frame, quality) = one_frame(args.screen, u64::from(index))?;
        let payload = tokio::task::spawn_blocking(move || encode_luma(&frame, quality))
            .await
            .map_err(|err| CliError::new(err.to_string()))?
            .map_err(|err| CliError::new(err.to_string()))?;
        let header = VideoFrameHeader {
            display_id: 1,
            codec: Codec::Av1,
            frame_no: u64::from(index),
            capture_ts_us: 1_000,
            keyframe: true,
            unknown: UnknownFields::new(),
        };
        link.send_view_frame(&header, &payload)
            .await
            .map_err(|err| CliError::new(err.to_string()))?;
    }
    Ok(())
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

async fn take_input(link: &mut cbdesk_net::HostLink, screen: bool) -> Result<(), CliError> {
    let event = tokio::time::timeout(Duration::from_secs(20), link.recv_input_event())
        .await
        .map_err(|_| CliError::new("timed out waiting for input"))?
        .map_err(|err| CliError::new(err.to_string()))?;
    if screen {
        inject_os(&event).map_err(|err| CliError::new(err.to_string()))?;
    } else {
        let mut log = InputLog::new();
        log.inject(&event)
            .map_err(|err| CliError::new(err.to_string()))?;
    }
    print_input(&event)?;
    Ok(())
}

fn print_input(event: &InputEvent) -> Result<(), CliError> {
    match &event.body {
        InputBody::Pointer(pointer) => println!("input pointer {} {}", pointer.x, pointer.y),
        InputBody::Key(key) => println!("input key {} {}", key.keycode, u8::from(key.down)),
    }
    io::stdout().flush()?;
    Ok(())
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

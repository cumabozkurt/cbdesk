// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dial a ticket and write one decoded frame.

use std::time::{SystemTime, UNIX_EPOCH};

use cbdesk_codec::decode_luma;
use cbdesk_crypto::Identity;
use cbdesk_discovery::ReplayCache;
use cbdesk_net::{
    dial_verified_ticket, open_controller, recv_view_frame, send_input_event, ControllerSession,
    EndpointConfig, QuicEndpoint,
};
use cbdesk_proto::{
    AuthMethod, Capability, Codec, Hello, InputBody, InputEvent, Permission, Platform,
    PointerEvent, ProtoMessage, UnknownFields, PROTO_MAJOR, PROTO_MINOR,
};
use zeroize::Zeroize;

use crate::args::ViewArgs;
use crate::error::CliError;
use crate::hex;
use crate::ppm::write_ppm;

/// Dial `args.ticket_hex` and write the first AV1 frame to `args.ppm`.
pub async fn view_remote(args: &ViewArgs) -> Result<(), CliError> {
    let bytes = hex::decode(args.ticket_hex.trim())?;
    let ticket = cbdesk_proto::InviteTicket::from_bytes(&bytes)
        .map_err(|err| CliError::new(err.to_string()))?;
    let (endpoint, _identity) = bind_viewer().await?;
    let mut replay = ReplayCache::new();
    let conn = dial_verified_ticket(&endpoint, &ticket, unix_now(), &mut replay)
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    let session = match open_controller(&conn, &hello(args.pointer.is_some()))
        .await
        .map_err(|err| CliError::new(err.to_string()))?
    {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => {
            return Err(CliError::new(format!("host closed: {:?}", close.code)));
        }
    };
    if !session.granted.perms.contains(&Permission::View) {
        return Err(CliError::new("host did not grant view"));
    }
    let (header, payload) = recv_view_frame(&conn)
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    if header.codec != Codec::Av1 {
        return Err(CliError::new("host sent a codec other than AV1"));
    }
    let frame = decode_luma(&payload).map_err(|err| CliError::new(err.to_string()))?;
    write_ppm(&args.ppm, &frame)?;
    if let Some((x, y)) = args.pointer {
        if !session.granted.perms.contains(&Permission::Input) {
            return Err(CliError::new("host did not grant input"));
        }
        let event = InputEvent {
            body: InputBody::Pointer(PointerEvent {
                x,
                y,
                buttons: 0,
                wheel: 0,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        };
        send_input_event(&conn, &event)
            .await
            .map_err(|err| CliError::new(err.to_string()))?;
    }
    let _ = session.release().await;
    Ok(())
}

async fn bind_viewer() -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = Identity::generate().map_err(|err| CliError::new(err.to_string()))?;
    let mut seed = identity.signing_seed();
    let addr = "127.0.0.1:0"
        .parse()
        .map_err(|_| CliError::new("viewer bind address is invalid"))?;
    let endpoint = QuicEndpoint::bind(&EndpointConfig::default(), addr, &seed)
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    seed.zeroize();
    Ok((endpoint, identity))
}

fn hello(want_input: bool) -> Hello {
    let mut wanted = vec![Permission::View];
    if want_input {
        wanted.push(Permission::Input);
    }
    Hello {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        app_version: "0.0.0".to_string(),
        platform: local_platform(),
        caps: vec![Capability::VideoAv1, Capability::Input],
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

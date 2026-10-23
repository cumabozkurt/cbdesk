// SPDX-License-Identifier: AGPL-3.0-or-later

//! Listen for one controller, and dial a ticket from this desk.
//!
//! The ticket is checked, then thrown away. Admission uses the consent
//! prompt. A short code never reaches this module.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use cbdesk_crypto::DeviceId;
use cbdesk_discovery::{sign_ticket, ReplayCache};
use cbdesk_net::{
    admit_invited, dial_verified_ticket, open_controller, recv_view_frame, ConsentAnswer,
    ControllerSession, EndpointConfig, HostLink, QuicConn, QuicEndpoint,
};
use cbdesk_proto::{
    local_capabilities, AuthMethod, Codec, Hello, InvitePurpose, InviteTicket, Permission,
    Platform, ProtoMessage, UnknownFields, VideoFrameHeader, PROTO_MAJOR, PROTO_MINOR,
};
use cbdesk_session::{PeerFacts, RateLimiter};
use tokio::runtime::Handle;
use zeroize::Zeroize;

use crate::error::DeskError;
use crate::hexutil;
use crate::preview::motion_pair;
use crate::state::{self, Inner};

pub async fn bind_endpoint(inner: &Mutex<Inner>) -> Result<QuicEndpoint, DeskError> {
    let mut seed = {
        let guard = lock(inner)?;
        guard.identity.signing_seed()
    };
    let addr: SocketAddr = "0.0.0.0:0"
        .parse()
        .map_err(|_| DeskError::Message("bind address is invalid".into()))?;
    let endpoint = QuicEndpoint::bind(&EndpointConfig::default(), addr, &seed)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    seed.zeroize();
    let port = endpoint
        .local_addr()
        .map_err(|err| DeskError::Message(err.to_string()))?
        .port();
    let mut guard = lock(inner)?;
    let signed = sign_ticket(
        InviteTicket {
            direct_addrs: direct_addrs(port),
            expires_at: unix_now().saturating_add(600),
            purpose: InvitePurpose::Session,
            ..InviteTicket::default()
        },
        &guard.identity,
    )
    .map_err(|err| DeskError::Message(err.to_string()))?;
    if !signed.pake_hint.is_empty() {
        return Err(DeskError::Message(
            "ticket carried a short-code hint".into(),
        ));
    }
    let bytes = signed
        .to_bytes()
        .map_err(|err| DeskError::Message(err.to_string()))?;
    guard.ticket_hex = hexutil::encode(&bytes);
    Ok(endpoint)
}

pub fn listen(handle: Handle, inner: Arc<Mutex<Inner>>, endpoint: QuicEndpoint) {
    handle.spawn(async move {
        while let Ok(conn) = endpoint.accept().await {
            if let Err(err) = serve_one(&inner, conn).await {
                if let Ok(mut guard) = inner.lock() {
                    guard.error = err.to_string();
                }
            }
        }
    });
}

pub fn arm(
    inner: &Mutex<Inner>,
    peer: &DeviceId,
    tx: tokio::sync::oneshot::Sender<ConsentAnswer>,
) -> Result<(), DeskError> {
    let mut guard = lock(inner)?;
    let now = state::clock_ms(&guard);
    let mut session = state::fresh_session()?;
    session
        .start_authentication(cbdesk_proto::AuthMethod::Interactive)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    session
        .authentication_succeeded(&PeerFacts { peer_known: false }, now)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    guard.session = session;
    guard.peer_id = peer.to_z32();
    guard.fingerprint = peer.short_fingerprint().to_string();
    guard.decision = Some(tx);
    guard.error.clear();
    let _ = state::ready_delay_ms();
    Ok(())
}

async fn serve_one(inner: &Mutex<Inner>, conn: QuicConn) -> Result<(), DeskError> {
    let peer = DeviceId::from_bytes(conn.remote_device_id())
        .map_err(|err| DeskError::Message(err.to_string()))?;
    if is_blocked(inner, &peer)? {
        return Err(DeskError::Blocked);
    }
    let now = clock(inner)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
    arm(inner, &peer, tx)?;
    {
        let mut guard = lock(inner)?;
        guard.cancel = Some(cancel_tx);
    }
    let source = IpAddr::from([127, 0, 0, 1]);
    let answer = async move {
        match rx.await {
            Ok(answer) => answer,
            Err(_) => ConsentAnswer::Reject { now_ms: now },
        }
    };
    let admitted = admit_invited(
        conn,
        state::fresh_session()?,
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        source,
        now,
        answer,
    )
    .await;
    let mut link = match admitted {
        Ok(link) => link,
        Err(failed) => {
            return match failed.error {
                cbdesk_net::AdmitError::Rejected | cbdesk_net::AdmitError::TimedOut => Ok(()),
                other => Err(DeskError::Message(other.to_string())),
            };
        }
    };
    let motion = motion_pair()?;
    send_frame(&mut link, &motion.key, 1, true).await?;
    send_frame(&mut link, &motion.inter, 2, false).await?;
    store_picture(inner, &motion.picture, true)?;
    tokio::select! {
        _ = link.read_control() => {}
        _ = cancel_rx => {}
    }
    finish_host(inner)?;
    Ok(())
}

pub async fn dial_and_show(
    inner: &Mutex<Inner>,
    endpoint: &QuicEndpoint,
    ticket: &InviteTicket,
) -> Result<(), DeskError> {
    let mut replay = ReplayCache::new();
    let conn = dial_verified_ticket(endpoint, ticket, unix_now(), &mut replay)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    let grant = match open_controller(&conn, &viewer_hello())
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?
    {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => {
            return Err(DeskError::Message(format!("host closed: {:?}", close.code)));
        }
    };
    let (_first, key_payload) = recv_view_frame(&conn)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    let (second, inter_payload) = recv_view_frame(&conn)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    if second.keyframe || !cbdesk_codec::vp8_is_inter(&inter_payload) {
        return Err(DeskError::Message(
            "remote picture was not an inter frame".into(),
        ));
    }
    let mut decoder = cbdesk_codec::Vp8StreamDecoder::new();
    decoder
        .decode(&key_payload)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    let picture = decoder
        .decode(&inter_payload)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    store_picture(inner, &picture, true)?;
    {
        let mut guard = lock(inner)?;
        guard.viewer_live = true;
    }
    grant
        .release()
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    Ok(())
}

async fn send_frame(
    link: &mut HostLink,
    payload: &[u8],
    frame_no: u64,
    keyframe: bool,
) -> Result<(), DeskError> {
    let header = VideoFrameHeader {
        display_id: 1,
        codec: Codec::Vp8,
        frame_no,
        capture_ts_us: 1_000,
        keyframe,
        unknown: UnknownFields::new(),
    };
    link.send_view_frame(&header, payload)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))
}

fn store_picture(
    inner: &Mutex<Inner>,
    picture: &cbdesk_capture::Picture,
    inter: bool,
) -> Result<(), DeskError> {
    let rgba = picture
        .to_rgba()
        .map_err(|err| DeskError::Message(err.to_string()))?;
    let mut guard = lock(inner)?;
    guard.frame_width = picture.width;
    guard.frame_height = picture.height;
    guard.frame_is_inter = inter;
    guard.frame_rgba = rgba;
    Ok(())
}

fn finish_host(inner: &Mutex<Inner>) -> Result<(), DeskError> {
    let mut guard = lock(inner)?;
    let _ = guard
        .session
        .close_with(cbdesk_session::CloseReason::LocalClose);
    Ok(())
}

fn is_blocked(inner: &Mutex<Inner>, peer: &DeviceId) -> Result<bool, DeskError> {
    let guard = lock(inner)?;
    Ok(guard.blocked.iter().any(|id| id == &peer.to_z32()))
}

fn clock(inner: &Mutex<Inner>) -> Result<u64, DeskError> {
    let guard = lock(inner)?;
    Ok(state::clock_ms(&guard))
}

fn lock(inner: &Mutex<Inner>) -> Result<std::sync::MutexGuard<'_, Inner>, DeskError> {
    inner
        .lock()
        .map_err(|_| DeskError::Message("desktop state lock was poisoned".into()))
}

fn viewer_hello() -> Hello {
    Hello {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        app_version: "0.0.0".into(),
        platform: Platform::Unspecified,
        caps: local_capabilities().to_vec(),
        wanted_perms: vec![Permission::View, Permission::Audio],
        auth_offer: vec![AuthMethod::Interactive],
        unknown: UnknownFields::new(),
        ..Hello::default()
    }
}

fn direct_addrs(port: u16) -> Vec<String> {
    let mut addrs = vec![format!("127.0.0.1:{port}")];
    if let Some(extra) = lan_ip(port) {
        if extra != addrs[0] {
            addrs.push(extra);
        }
    }
    addrs
}

fn lan_ip(port: u16) -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_loopback() || ip.is_unspecified() {
        return None;
    }
    Some(SocketAddr::from((ip, port)).to_string())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

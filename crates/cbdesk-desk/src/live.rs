// SPDX-License-Identifier: AGPL-3.0-or-later

//! Listen for one controller, and dial a ticket from this desk.
//!
//! The ticket is checked, then thrown away. A desk host runs SPAKE2 on the
//! first stream before it shows the consent prompt. Admission still uses
//! that prompt. A short code alone does not locate a peer.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cbdesk_crypto::{password_bytes, DeviceId};
use cbdesk_discovery::{sign_ticket, ReplayCache, ShortCode};
use cbdesk_net::{
    accept_pairing, admit_invited, dial_verified_ticket, offer_pairing, open_controller,
    ConsentAnswer, ControllerSession, EndpointConfig, Pairing, QuicConn, QuicEndpoint,
};
use cbdesk_proto::{
    local_capabilities, AuthMethod, Hello, InvitePurpose, InviteTicket, Permission, Platform,
    ProtoMessage, UnknownFields, PROTO_MAJOR, PROTO_MINOR,
};
use cbdesk_session::{PeerFacts, RateLimiter};
use tokio::runtime::Handle;
use zeroize::Zeroize;

use crate::error::DeskError;
use crate::hexutil;
use crate::session;
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
    if let Err(err) = pair_host(inner, &conn).await {
        note_error(inner, &err.to_string());
        return Err(err);
    }
    let now = clock(inner)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    arm(inner, &peer, tx)?;
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
    let link = match admitted {
        Ok(link) => link,
        Err(failed) => {
            return match failed.error {
                cbdesk_net::AdmitError::Rejected | cbdesk_net::AdmitError::TimedOut => Ok(()),
                other => Err(DeskError::Message(other.to_string())),
            };
        }
    };
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    let session_id = session::install(inner, cmd_tx, cancel_tx)?;
    session::host_session(inner, link, cmd_rx, cancel_rx, session_id).await
}

async fn pair_host(inner: &Mutex<Inner>, conn: &QuicConn) -> Result<(), DeskError> {
    let pairing = host_pairing(inner)?;
    let paired =
        tokio::time::timeout(Duration::from_secs(20), accept_pairing(conn, &pairing)).await;
    match paired {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) | Err(_) => Err(DeskError::Message("pairing failed".into())),
    }
}

fn host_pairing(inner: &Mutex<Inner>) -> Result<Pairing, DeskError> {
    let guard = lock(inner)?;
    Ok(Pairing::host(
        password_bytes(&guard.short_code.words),
        guard.short_code.slot,
        *guard.identity.device_id().as_bytes(),
    ))
}

pub async fn dial_and_show(
    inner: Arc<Mutex<Inner>>,
    endpoint: &QuicEndpoint,
    ticket: &InviteTicket,
    code: Option<ShortCode>,
) -> Result<(), DeskError> {
    let mut replay = ReplayCache::new();
    let conn = dial_verified_ticket(endpoint, ticket, unix_now(), &mut replay)
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?;
    if let Some(code) = code.as_ref() {
        pair_controller(&inner, &conn, code).await?;
    }
    let grant = match open_controller(&conn, &viewer_hello())
        .await
        .map_err(|err| DeskError::Message(err.to_string()))?
    {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => {
            return Err(DeskError::Message(format!("host closed: {:?}", close.code)));
        }
    };
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    let session_id = session::install(&inner, cmd_tx, cancel_tx)?;
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let task_inner = Arc::clone(&inner);
    tokio::spawn(async move {
        session::viewer_session(
            task_inner,
            conn,
            grant,
            cmd_rx,
            cancel_rx,
            Some(ready_tx),
            session_id,
        )
        .await;
    });
    match tokio::time::timeout(Duration::from_secs(30), ready_rx).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(err))) => Err(DeskError::Message(err)),
        Ok(Err(_)) => Err(DeskError::Message(
            "session ended before an inter frame".into(),
        )),
        Err(_) => Err(DeskError::Message(
            "timed out waiting for an inter frame".into(),
        )),
    }
}

async fn pair_controller(
    inner: &Mutex<Inner>,
    conn: &QuicConn,
    code: &ShortCode,
) -> Result<(), DeskError> {
    let pairing = {
        let guard = lock(inner)?;
        Pairing::controller(
            password_bytes(&code.words),
            code.slot,
            *guard.identity.device_id().as_bytes(),
        )
    };
    let paired = tokio::time::timeout(Duration::from_secs(20), offer_pairing(conn, &pairing)).await;
    match paired {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) | Err(_) => Err(DeskError::Message("pairing failed".into())),
    }
}

fn note_error(inner: &Mutex<Inner>, text: &str) {
    if let Ok(mut guard) = inner.lock() {
        guard.error = text.to_string();
    }
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
        wanted_perms: vec![
            Permission::View,
            Permission::Input,
            Permission::Clipboard,
            Permission::Audio,
            Permission::FileRead,
            Permission::FileWrite,
        ],
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

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Loopback ticket dial into a host session. Not a screen capture.

use std::future::{pending, ready};
use std::net::SocketAddr;

use cbdesk_crypto::Identity;
use cbdesk_discovery::{sign_ticket, ticket_grants_access, ReplayCache, MAX_TICKET_TTL_SECS};
use cbdesk_proto::{
    AuthMethod, Control, ControlBody, ErrorCode, Hello, InvitePurpose, InviteTicket, Permission,
    ProtoMessage, PROTO_MAJOR, PROTO_MINOR,
};
use cbdesk_session::{
    ApprovalMode, HostSession, PeerFacts, Phase, RateLimiter, SecurityConfig, CONSENT_DELAY_MS,
};

use crate::admit::{admit_until, ConsentWait};
use crate::dial::dial_verified_ticket;
use crate::policy::EndpointConfig;
use crate::quic::QuicEndpoint;
use crate::{AdmitError, ConsentAnswer, ControllerOutcome, DialError, NetError};

fn seed(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn always() -> SecurityConfig {
    SecurityConfig {
        approval: ApprovalMode::Always,
        unattended_enabled: false,
        require_2fa: false,
        quick_support: false,
    }
}

fn hello() -> Hello {
    Hello {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        app_version: "0.0.0".to_string(),
        wanted_perms: vec![Permission::View, Permission::FileWrite],
        auth_offer: vec![AuthMethod::Interactive],
        ..Hello::default()
    }
}

async fn bind(byte: u8) -> QuicEndpoint {
    let addr: SocketAddr = "127.0.0.1:0".parse().expect("addr");
    QuicEndpoint::bind(&EndpointConfig::default(), addr, &seed(byte))
        .await
        .expect("bind")
}

fn signed(identity: &Identity, addrs: Vec<String>, expires_at: u64) -> InviteTicket {
    sign_ticket(
        InviteTicket {
            direct_addrs: addrs,
            relay_urls: vec!["https://foo.iroh.link/relay".to_string()],
            expires_at,
            purpose: InvitePurpose::Session,
            ..InviteTicket::default()
        },
        identity,
    )
    .expect("sign")
}

fn decode_frame(bytes: &[u8]) -> Control {
    let len = u32::from_be_bytes(bytes[..4].try_into().expect("len")) as usize;
    Control::from_bytes(&bytes[4..4 + len]).expect("control")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn verified_ticket_reaches_host_only_after_consent() {
    let host_identity = Identity::from_seed(&seed(21)).expect("host");
    let host = bind(21).await;
    let client = bind(22).await;
    let host_addr = host.local_addr().expect("addr");
    let ticket = signed(
        &host_identity,
        vec!["192.0.2.1:22116".to_string(), host_addr.to_string()],
        50_000,
    );
    let mut replay = ReplayCache::new();
    let (dialed, accepted) = tokio::join!(
        dial_verified_ticket(&client, &ticket, 50_000 - 60, &mut replay),
        host.accept(),
    );
    let dialed = dialed.expect("dial");
    let accepted = accepted.expect("accept");
    assert!(client.dns_queries().is_empty());
    assert!(host.dns_queries().is_empty());
    assert!(host.relay_urls().is_empty());
    assert!(!ticket_grants_access());
    let second = dial_verified_ticket(&client, &ticket, 50_000 - 60, &mut replay).await;
    assert!(matches!(
        second,
        Err(DialError::Discovery(
            cbdesk_discovery::DiscoveryError::TicketReplayed
        ))
    ));

    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(ConsentAnswer::Approve {
        now_ms: CONSENT_DELAY_MS,
        perms: vec![Permission::View],
        explicit_high_risk: false,
    })
    .expect("answer");
    let answer = async move { rx.await.expect("answer") };
    let waits = ConsentWait {
        answer,
        timeout: pending::<()>(),
    };
    let admit = admit_until(
        accepted,
        HostSession::new(always()).expect("session"),
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        host_addr.ip(),
        0,
        waits,
    );
    let hello = hello();
    let grant = crate::request_grant(&dialed, &hello);
    let (link, outcome) = tokio::join!(admit, grant);
    let link = link.expect("admit");
    match outcome.expect("grant") {
        ControllerOutcome::Granted(granted) => {
            assert_eq!(granted.perms, vec![Permission::View]);
            assert_eq!(granted.session_id.len(), 16);
            assert!(granted.reconnect_token.is_empty());
        }
        ControllerOutcome::Closed(close) => panic!("closed {:?}", close.code),
    }
    link.authorize(Permission::View).expect("view");
    assert!(link.authorize(Permission::FileWrite).is_err());
    assert!(matches!(
        link.session().phase(),
        Phase::Active {
            media_started: false,
            ..
        }
    ));
    assert!(link.session().indicator_visible());
    assert!(!HostSession::indicator_can_be_disabled());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unanswered_prompt_sends_timeout_user_response() {
    let host_identity = Identity::from_seed(&seed(23)).expect("host");
    let host = bind(23).await;
    let client = bind(24).await;
    let host_addr = host.local_addr().expect("addr");
    let ticket = signed(&host_identity, vec![host_addr.to_string()], 80_000);
    let mut replay = ReplayCache::new();
    let (dialed, accepted) = tokio::join!(
        dial_verified_ticket(&client, &ticket, 80_000 - 30, &mut replay),
        host.accept(),
    );
    let waits = ConsentWait {
        answer: pending::<ConsentAnswer>(),
        timeout: ready(()),
    };
    let admit = admit_until(
        accepted.expect("accept"),
        HostSession::new(always()).expect("session"),
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        host_addr.ip(),
        0,
        waits,
    );
    let dialed = dialed.expect("dial");
    let hello = hello();
    let grant = crate::request_grant(&dialed, &hello);
    let (failed, outcome) = tokio::join!(admit, grant);
    let mut failed = failed.expect_err("timeout");
    assert_eq!(failed.error, AdmitError::TimedOut);
    assert!(matches!(
        failed.session.phase(),
        Phase::Closed {
            reason: cbdesk_session::CloseReason::TimedOut
        }
    ));
    assert!(!failed.session.indicator_visible());
    assert!(failed.session.authorize(Permission::View).is_err());
    assert!(failed.session.open_media().is_err());
    match outcome.expect("close") {
        ControllerOutcome::Closed(close) => {
            assert_eq!(close.code, ErrorCode::TimeoutUserResponse);
        }
        ControllerOutcome::Granted(_) => panic!("granted without an answer"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_probe_does_not_grant_or_echo() {
    let host = bind(25).await;
    let client = bind(26).await;
    let host_addr = host.local_addr().expect("addr");
    let host_key = host.public_key();
    let probe = async {
        let conn = client.connect(&host_key, host_addr).await?;
        conn.probe_send(b"cbdesk", 64).await
    };
    let admit = async {
        let conn = host.accept().await.expect("accept");
        let waits = ConsentWait {
            answer: pending::<ConsentAnswer>(),
            timeout: pending::<()>(),
        };
        admit_until(
            conn,
            HostSession::new(always()).expect("session"),
            PeerFacts { peer_known: false },
            RateLimiter::new(),
            host_addr.ip(),
            0,
            waits,
        )
        .await
    };
    let (reply, failed) = tokio::join!(probe, admit);
    let mut failed = failed.expect_err("probe");
    assert!(matches!(
        failed.error,
        AdmitError::Net(NetError::BadControl)
    ));
    assert!(matches!(failed.session.phase(), Phase::Idle));
    assert!(!failed.session.indicator_visible());
    assert!(failed.session.open_media().is_err());
    assert!(!ticket_grants_access());
    let reply = reply.expect("frame");
    assert_ne!(reply, b"cbdesk");
    match decode_frame(&reply).body {
        ControlBody::Close(close) => assert_eq!(close.code, ErrorCode::ProtocolViolation),
        other => panic!("probe was treated as a session: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bad_tickets_fail_before_a_grant() {
    let host_identity = Identity::from_seed(&seed(27)).expect("host");
    let client = bind(28).await;
    let mut replay = ReplayCache::new();
    let expired = signed(&host_identity, vec!["127.0.0.1:22116".to_string()], 10);
    let first = dial_verified_ticket(&client, &expired, 10, &mut replay).await;
    let second = dial_verified_ticket(&client, &expired, 10, &mut replay).await;
    assert!(matches!(
        first,
        Err(DialError::Discovery(
            cbdesk_discovery::DiscoveryError::TicketExpired
        ))
    ));
    assert!(matches!(
        second,
        Err(DialError::Discovery(
            cbdesk_discovery::DiscoveryError::TicketExpired
        ))
    ));
    let mut tampered = signed(
        &host_identity,
        vec!["127.0.0.1:9".to_string()],
        10 + MAX_TICKET_TTL_SECS,
    );
    tampered.direct_addrs[0] = "127.0.0.1:10".to_string();
    assert!(dial_verified_ticket(&client, &tampered, 10, &mut replay)
        .await
        .is_err());
    assert!(client.dns_queries().is_empty());
}

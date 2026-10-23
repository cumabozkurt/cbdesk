// SPDX-License-Identifier: AGPL-3.0-or-later

//! View frames and input events on loopback QUIC, after consent.

use std::future::pending;
use std::net::SocketAddr;
use std::time::Duration;

use cbdesk_proto::{
    AuthMethod, Codec, Control, ControlBody, ErrorCode, Hello, InputBody, InputEvent, Permission,
    PointerEvent, UnknownFields, VideoFrameHeader, PROTO_MAJOR, PROTO_MINOR,
};
use cbdesk_session::{
    ApprovalMode, HostSession, PeerFacts, Phase, RateLimiter, SecurityConfig, SessionError,
    CONSENT_DELAY_MS,
};

use crate::admit::{admit_until, open_controller, ConsentWait};
use crate::control::ControlChannel;
use crate::policy::EndpointConfig;
use crate::quic::QuicEndpoint;
use crate::{recv_view_frame, send_input_event, AdmitError, ConsentAnswer, ControllerSession};

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

fn approve(perms: Vec<Permission>) -> ConsentAnswer {
    ConsentAnswer::Approve {
        now_ms: CONSENT_DELAY_MS,
        perms,
        explicit_high_risk: false,
    }
}

struct Linked {
    link: crate::HostLink,
    dialed: crate::QuicConn,
    grant: crate::ControllerGrant,
    // The endpoint driver dies when the last handle is dropped. The
    // connections above are not enough to keep it running.
    _host: QuicEndpoint,
    _client: QuicEndpoint,
}

async fn consented(host_byte: u8, client_byte: u8, perms: Vec<Permission>) -> Linked {
    let host = bind(host_byte).await;
    let client = bind(client_byte).await;
    let host_addr = host.local_addr().expect("addr");
    let host_key = host.public_key();
    let (dialed, accepted) = tokio::join!(client.connect(&host_key, host_addr), host.accept(),);
    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(approve(perms)).expect("answer");
    let admit = admit_until(
        accepted.expect("accept"),
        HostSession::new(always()).expect("session"),
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        host_addr.ip(),
        0,
        ConsentWait {
            answer: async move { rx.await.expect("answer") },
            timeout: pending::<()>(),
        },
    );
    let dialed = dialed.expect("dial");
    let hello = hello();
    let opened = open_controller(&dialed, &hello);
    let (link, opened) = tokio::join!(admit, opened);
    let grant = match opened.expect("controller") {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => panic!("closed {:?}", close.code),
    };
    Linked {
        link: link.expect("admit"),
        dialed,
        grant,
        _host: host,
        _client: client,
    }
}

fn header() -> VideoFrameHeader {
    VideoFrameHeader {
        display_id: 1,
        codec: Codec::Av1,
        frame_no: 3,
        capture_ts_us: 8,
        keyframe: true,
        unknown: UnknownFields::new(),
    }
}

fn pointer(x: i32, y: i32) -> InputEvent {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn side_stream_before_grant_is_protocol_violation() {
    let host = bind(31).await;
    let client = bind(32).await;
    let host_addr = host.local_addr().expect("addr");
    let host_key = host.public_key();
    let (dialed, accepted) = tokio::join!(client.connect(&host_key, host_addr), host.accept());
    let dialed = dialed.expect("dial");
    let admit = admit_until(
        accepted.expect("accept"),
        HostSession::new(always()).expect("session"),
        PeerFacts { peer_known: false },
        RateLimiter::new(),
        host_addr.ip(),
        0,
        ConsentWait {
            answer: pending::<ConsentAnswer>(),
            timeout: pending::<()>(),
        },
    );
    let early = async {
        let mut channel = ControlChannel::open(&dialed).await.expect("control");
        channel
            .write(&Control {
                body: ControlBody::Hello(hello()),
                unknown: UnknownFields::new(),
            })
            .await
            .expect("hello");
        let _ = channel.read().await.expect("ack");
        let mut uni = dialed.open_uni_stream().await.expect("uni");
        let body = b"early";
        uni.write_all(&(body.len() as u32).to_be_bytes())
            .await
            .expect("len");
        uni.write_all(body).await.expect("body");
        uni.finish().expect("finish");
        channel.read().await.expect("close")
    };
    let (failed, message) = tokio::join!(admit, early);
    let mut failed = failed.expect_err("side stream");
    assert_eq!(
        failed.error,
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    );
    assert!(failed.session.authorize(Permission::View).is_err());
    assert!(failed.session.open_media().is_err());
    match message.body {
        ControlBody::Close(close) => assert_eq!(close.code, ErrorCode::ProtocolViolation),
        other => panic!("side stream was granted: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_frame_follows_a_view_grant() {
    let mut linked = consented(33, 34, vec![Permission::View]).await;
    let payload = b"barcode-bytes".to_vec();
    let header = header();
    let send = linked.link.send_view_frame(&header, &payload);
    let recv = recv_view_frame(&linked.dialed);
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    let (got_header, got_payload) = got.expect("frame");
    assert_eq!(got_payload, payload);
    assert_eq!(got_header.frame_no, 3);
    assert_eq!(got_header.codec, Codec::Av1);
    assert!(got_header.keyframe);
    assert!(matches!(
        linked.link.session().phase(),
        Phase::Active {
            media_started: true,
            ..
        }
    ));
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn input_grant_does_not_open_a_view_stream() {
    let mut linked = consented(35, 36, vec![Permission::Input]).await;
    let error = linked
        .link
        .send_view_frame(&header(), b"nope")
        .await
        .expect_err("view");
    assert_eq!(error, AdmitError::Session(SessionError::PermissionDenied));
    let waited =
        tokio::time::timeout(Duration::from_millis(400), recv_view_frame(&linked.dialed)).await;
    assert!(waited.is_err(), "a view stream opened without View");
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn input_without_the_grant_is_protocol_violation() {
    let mut linked = consented(37, 38, vec![Permission::View]).await;
    let event = pointer(4, 5);
    let send = send_input_event(&linked.dialed, &event);
    let recv = linked.link.recv_input_event();
    let (sent, got) = tokio::join!(send, recv);
    let _ = sent;
    assert_eq!(
        got.expect_err("input"),
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    );
    match linked.grant.read().await.expect("close").body {
        ControlBody::Close(close) => assert_eq!(close.code, ErrorCode::ProtocolViolation),
        other => panic!("input was applied: {other:?}"),
    }
    assert!(matches!(
        linked.link.session().phase(),
        Phase::Closed { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn granted_input_returns_the_event_and_does_not_need_the_os() {
    let mut linked = consented(39, 40, vec![Permission::View, Permission::Input]).await;
    let event = pointer(12, 34);
    let send = send_input_event(&linked.dialed, &event);
    let recv = linked.link.recv_input_event();
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    assert_eq!(got.expect("event"), event);
    drop(linked.grant);
}

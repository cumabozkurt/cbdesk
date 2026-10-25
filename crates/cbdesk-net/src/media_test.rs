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
use crate::{
    recv_view_frame, send_clipboard_text, send_input_event, AdmitError, ConsentAnswer,
    ControllerSession,
};

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

fn approve_risk(perms: Vec<Permission>, explicit_high_risk: bool) -> ConsentAnswer {
    ConsentAnswer::Approve {
        now_ms: CONSENT_DELAY_MS,
        perms,
        explicit_high_risk,
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
    consented_risk(host_byte, client_byte, perms, false).await
}

async fn consented_risk(
    host_byte: u8,
    client_byte: u8,
    perms: Vec<Permission>,
    high_risk: bool,
) -> Linked {
    let host = bind(host_byte).await;
    let client = bind(client_byte).await;
    let host_addr = host.local_addr().expect("addr");
    let host_key = host.public_key();
    let (dialed, accepted) = tokio::join!(client.connect(&host_key, host_addr), host.accept(),);
    let (tx, rx) = tokio::sync::oneshot::channel();
    tx.send(approve_risk(perms, high_risk)).expect("answer");
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
        damage_x: 0,
        damage_y: 0,
        damage_width: 0,
        damage_height: 0,
        unchanged: false,
        ..VideoFrameHeader::default()
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_text_follows_the_grant() {
    let mut linked = consented(41, 42, vec![Permission::View, Permission::Clipboard]).await;
    let text = cbdesk_proto::ClipboardText {
        text: "merhaba".to_string(),
        generation: 2,
        unknown: UnknownFields::new(),
    };
    let send = send_clipboard_text(&linked.dialed, &text);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    match got.expect("clipboard") {
        crate::PeerEvent::Clipboard(body) => assert_eq!(body, text),
        other => panic!("expected clipboard text, got {other:?}"),
    }
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_without_the_grant_is_protocol_violation() {
    let mut linked = consented(43, 44, vec![Permission::View]).await;
    let text = cbdesk_proto::ClipboardText {
        text: "nope".to_string(),
        generation: 1,
        unknown: UnknownFields::new(),
    };
    let send = send_clipboard_text(&linked.dialed, &text);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    let _ = sent;
    assert_eq!(
        got.expect_err("clipboard"),
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    );
    drop(linked.grant);
}

fn sample_chunk() -> cbdesk_proto::FileChunk {
    cbdesk_proto::FileChunk {
        transfer: 1,
        path: "note.txt".to_string(),
        offset: 0,
        data: b"hello".to_vec(),
        hash: vec![9; 32],
        total: 5,
        last: true,
        root: vec![8; 32],
        compressed: false,
        chunk_size: 256 * 1024,
        unknown: UnknownFields::new(),
    }
}

fn sample_feedback() -> cbdesk_proto::Feedback {
    cbdesk_proto::Feedback {
        recv_kbps: 100,
        loss: 0.0,
        frame_delay_ms: 10,
        last_frame_no: 1,
        unknown: UnknownFields::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_chunk_follows_a_file_write_grant() {
    let mut linked = consented_risk(45, 46, vec![Permission::FileWrite], true).await;
    let chunk = sample_chunk();
    let send = crate::send_file_chunk(&linked.dialed, &chunk);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    match got.expect("file") {
        crate::PeerEvent::File(body) => assert_eq!(body, chunk),
        other => panic!("expected a file chunk, got {other:?}"),
    }
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_chunk_without_the_grant_is_protocol_violation() {
    let mut linked = consented(47, 48, vec![Permission::View]).await;
    let chunk = sample_chunk();
    let send = crate::send_file_chunk(&linked.dialed, &chunk);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    let _ = sent;
    assert_eq!(
        got.expect_err("file"),
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    );
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_file_chunk_follows_a_file_read_grant() {
    let mut linked = consented(49, 50, vec![Permission::FileRead]).await;
    let chunk = sample_chunk();
    let send = linked.link.send_file_chunk(&chunk);
    let recv = crate::recv_uni(&linked.dialed);
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    match got.expect("file") {
        crate::UniBody::File(body) => assert_eq!(body, chunk),
        other => panic!("expected a file chunk, got {other:?}"),
    }
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audio_packet_follows_an_audio_grant() {
    let mut linked = consented(51, 52, vec![Permission::Audio]).await;
    let packet = b"tone".to_vec();
    let send = crate::send_audio_packet(&linked.dialed, &packet);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    match got.expect("audio") {
        crate::PeerEvent::Audio(body) => assert_eq!(body, packet),
        other => panic!("expected audio, got {other:?}"),
    }
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audio_without_the_grant_is_protocol_violation() {
    let mut linked = consented(53, 54, vec![Permission::View]).await;
    let send = crate::send_audio_packet(&linked.dialed, b"nope");
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    let _ = sent;
    assert_eq!(
        got.expect_err("audio"),
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    );
    drop(linked.grant);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feedback_follows_a_view_grant() {
    let mut linked = consented(55, 56, vec![Permission::View]).await;
    let feedback = sample_feedback();
    let send = crate::send_feedback(&linked.dialed, &feedback);
    let recv = linked.link.next_peer_event();
    let (sent, got) = tokio::join!(send, recv);
    sent.expect("send");
    match got.expect("feedback") {
        crate::PeerEvent::Feedback(body) => assert_eq!(body, feedback),
        other => panic!("expected feedback, got {other:?}"),
    }
    drop(linked.grant);
}

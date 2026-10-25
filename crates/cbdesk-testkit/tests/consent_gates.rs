// SPDX-License-Identifier: AGPL-3.0-or-later

//! Consent gates a session, and the memory transport can carry a Hello.

use cbdesk_crypto::Identity;
use cbdesk_discovery::ticket_grants_access;
use cbdesk_net::{MemoryHub, Transport};
use cbdesk_proto::{supports_hidden_session, Hello, Permission, ProtoMessage, PROTO_MAJOR};
use cbdesk_session::{ApprovalMode, HostSession, PeerFacts, SecurityConfig};
use cbdesk_testkit::{RecordingInjector, SyntheticCapturer, BARCODE_WIDTH};

#[tokio::test]
async fn consent_gates_session_and_memory_carries_hello() {
    assert!(!supports_hidden_session());
    assert!(!ticket_grants_access());
    assert!(!HostSession::indicator_can_be_disabled());

    let config = SecurityConfig {
        approval: ApprovalMode::Always,
        unattended_enabled: false,
        require_2fa: false,
        quick_support: true,
    };
    let mut host = HostSession::new(config).expect("config");
    assert!(host.open_media().is_err());
    host.start_authentication(cbdesk_proto::AuthMethod::Interactive)
        .expect("start");
    host.authentication_succeeded(&PeerFacts { peer_known: false }, 0)
        .expect("auth");
    assert!(host.indicator_visible());
    assert!(host.approve(0, &[Permission::View], false).is_err());
    host.approve(3_000, &[Permission::View], false)
        .expect("approve");
    host.authorize(Permission::View).expect("view");
    assert!(host.authorize(Permission::FileWrite).is_err());
    host.open_media().expect("media");

    let hub = MemoryHub::new();
    let left_id = Identity::generate().expect("left").device_id();
    let right_id = Identity::generate().expect("right").device_id();
    let dialer = hub.bind(left_id).expect("bind");
    let acceptor = hub.bind(right_id).expect("bind");
    let outbound = Transport::dial(&dialer, right_id, &[]).await.expect("dial");
    let hello = Hello {
        proto_major: PROTO_MAJOR,
        app_version: "0.0.0".to_string(),
        ..Hello::default()
    };
    outbound
        .send(hello.to_bytes().expect("encode"))
        .await
        .expect("send");
    let mut inbound = Transport::accept(&acceptor).await.expect("accept");
    let got = Hello::from_bytes(&inbound.recv().await.expect("recv")).expect("decode");
    assert_eq!(got.proto_major, PROTO_MAJOR);

    let frame = SyntheticCapturer::new(BARCODE_WIDTH, 1)
        .capture(3, 5)
        .expect("frame");
    let mut recording = RecordingInjector::new();
    recording.push(frame);
    assert_eq!(recording.len(), 1);
    assert!(recording.inject().is_some());
    assert!(recording.is_empty());
}

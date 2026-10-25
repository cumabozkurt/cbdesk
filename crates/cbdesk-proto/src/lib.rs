// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! CBDesk session protocol, version `cbdesk/1`.
//!
//! This crate is the implemented subset of the protocol: handshake messages,
//! grants, invite tickets, DHT address records, input events, clipboard text,
//! and the size limits parsers must enforce. Encoded video bytes are not a
//! protobuf field.
//! The readable schema is `proto/cbdesk/v1/session.proto`.
//!
//! # Consent
//!
//! An [`InviteTicket`] tells a controller where a host might be. It does not
//! grant control. A [`Granted`] message is produced only after the host
//! session layer accepts an interactive approval or an explicit unattended
//! policy. No message in this crate can ask a peer to hide that session.

mod clipboard;
mod control;
mod enums;
mod error;
mod file;
mod input;
mod limits;
mod messages;
mod negotiate;
mod wire;

pub use clipboard::{ClipboardText, CLIPBOARD_TEXT_MAX_BYTES};
pub use control::{Control, ControlBody};
pub use enums::{AuthMethod, Capability, Codec, ErrorCode, InvitePurpose, Permission, Platform};
pub use error::ProtoError;
pub use file::{FileChunk, FILE_CHUNK_MAX_BYTES, FILE_MESSAGE_MAX_BYTES};
pub use input::{InputBody, InputEvent, KeyEvent, PointerEvent};
pub use limits::{
    check_payload, PayloadKind, CLIPBOARD_ABSOLUTE_MAX_BYTES, CLIPBOARD_DEFAULT_MAX_BYTES,
    CONTROL_MAX_BYTES, MAX_DIRECT_ADDRS, VIDEO_FRAME_MAX_BYTES,
};
pub use messages::{
    Close, DhtAddressRecord, DisplayInfo, Feedback, Granted, Hello, HelloAck, InviteTicket,
    PermissionChanged, ProtoMessage, SignedDhtRecord, VideoFrameHeader,
};
pub use negotiate::{local_capabilities, select_video_codec};
pub use wire::{FieldValue, UnknownFields};

/// ALPN token for this major version.
pub const ALPN: &str = "cbdesk/1";

/// Major version implemented by this crate.
pub const PROTO_MAJOR: u32 = 1;

/// Minor version implemented by this crate.
pub const PROTO_MINOR: u32 = 0;

/// User-interface catalog keys that must exist in every shipped language.
pub const UI_MESSAGE_KEYS: &[&str] = &[
    "consent.title",
    "consent.body",
    "consent.accept",
    "consent.reject",
    "consent.delay_hint",
    "fraud.warning",
    "session.indicator",
    "session.summary",
    "quick_support.no_unattended",
];

/// True when two peers are allowed to attempt a connection under the N / N-1 rule.
///
/// Major version 0 is never compatible. This crate can *speak* only
/// [`PROTO_MAJOR`]; the helper exists so the policy is tested before a second
/// major version exists.
pub fn major_compatible(local: u32, remote: u32) -> bool {
    if local == 0 || remote == 0 {
        return false;
    }
    local == remote || local + 1 == remote || remote + 1 == local
}

/// Whether this build can run a session at `remote_major`.
pub fn session_supports(remote_major: u32) -> bool {
    remote_major == PROTO_MAJOR
}

/// The protocol has no hidden-session mode. Callers cannot turn this on.
pub const fn supports_hidden_session() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn roundtrip<T: ProtoMessage + std::fmt::Debug>(message: &T) {
        let bytes = message.to_bytes().expect("encode");
        let back = T::from_bytes(&bytes).expect("decode");
        assert_eq!(&back, message);
    }

    #[test]
    fn empty_hello_is_default() {
        let hello = Hello::from_bytes(&[]).expect("empty");
        assert_eq!(hello, Hello::default());
    }

    #[test]
    fn messages_roundtrip() {
        roundtrip(&Hello {
            proto_major: PROTO_MAJOR,
            proto_minor: 4,
            app_version: "0.0.0".to_string(),
            platform: Platform::Linux,
            caps: vec![Capability::VideoVp8, Capability::Unknown(40)],
            wanted_perms: vec![Permission::View, Permission::Input],
            auth_offer: vec![AuthMethod::Interactive],
            device_name_enc: b"cipher".to_vec(),
            unknown: unknown_len(99, b"ext"),
        });
        roundtrip(&HelloAck {
            proto_major: 1,
            proto_minor: 0,
            caps: vec![Capability::Relay],
            auth_required: vec![AuthMethod::Interactive, AuthMethod::Totp],
            unknown: UnknownFields::new(),
        });
        roundtrip(&DisplayInfo {
            id: 2,
            name: "HDMI-1".to_string(),
            x: -20,
            y: 5,
            width: 1920,
            height: 1080,
            scale: 1.5,
            primary: true,
            hdr: false,
            unknown: UnknownFields::new(),
        });
        roundtrip(&Granted {
            perms: vec![Permission::View],
            expires_at: 1_800_000_000,
            session_id: vec![9; 16],
            displays: vec![DisplayInfo {
                id: 1,
                width: 800,
                height: 600,
                scale: 1.0,
                ..DisplayInfo::default()
            }],
            reconnect_token: b"tok".to_vec(),
            unknown: UnknownFields::new(),
        });
        roundtrip(&PermissionChanged {
            perms: vec![Permission::View],
            reason: "user".to_string(),
            unknown: UnknownFields::new(),
        });
        roundtrip(&VideoFrameHeader {
            display_id: 1,
            codec: Codec::Vp8,
            frame_no: 42,
            capture_ts_us: 99,
            keyframe: true,
            damage_x: 16,
            damage_y: 32,
            damage_width: 64,
            damage_height: 48,
            unchanged: true,
            unknown: UnknownFields::new(),
        });
        roundtrip(&Feedback {
            recv_kbps: 2_000,
            loss: 0.125,
            frame_delay_ms: 8,
            last_frame_no: 41,
            unknown: UnknownFields::new(),
        });
        roundtrip(&Close {
            code: ErrorCode::RejectedByUser,
            detail: "no".to_string(),
            unknown: UnknownFields::new(),
        });
        roundtrip(&InviteTicket {
            version: 1,
            peer_pk: vec![7; 32],
            direct_addrs: vec![
                "192.0.2.10:22116".to_string(),
                "[2001:db8::1]:22116".to_string(),
            ],
            relay_urls: vec!["relay://example.test:443".to_string()],
            expires_at: 1_700_000_000,
            purpose: InvitePurpose::Session,
            pake_hint: b"hint".to_vec(),
            sig: vec![1; 64],
            unknown: unknown_len(50, b"later"),
        });
        roundtrip(&DhtAddressRecord {
            direct_addrs: vec!["203.0.113.5:22116".to_string()],
            relay_urls: vec!["relay://relay.example:443".to_string()],
            protocol_version: 1,
            timestamp: 1_700_000_100,
            publish_ip: true,
            unknown: UnknownFields::new(),
        });
        roundtrip(&SignedDhtRecord {
            signer_pk: vec![3; 32],
            record: b"record-bytes".to_vec(),
            sig: vec![4; 64],
            unknown: UnknownFields::new(),
        });
    }

    #[test]
    fn unknown_permission_value_is_preserved() {
        // Field 6, packed, one varint: 99.
        let bytes = [0x32, 0x01, 99];
        let hello = Hello::from_bytes(&bytes).expect("decode");
        assert_eq!(hello.wanted_perms, vec![Permission::Unknown(99)]);
        let again = Hello::from_bytes(&hello.to_bytes().expect("encode")).expect("redecode");
        assert_eq!(again.wanted_perms, hello.wanted_perms);
    }

    #[test]
    fn unpacked_repeated_permissions_are_accepted() {
        // Field 6 as two unpacked varints: View, Input.
        let bytes = [0x30, 0x01, 0x30, 0x02];
        let hello = Hello::from_bytes(&bytes).expect("decode");
        assert_eq!(
            hello.wanted_perms,
            vec![Permission::View, Permission::Input]
        );
    }

    #[test]
    fn singular_field_last_value_wins() {
        let hello = Hello::from_bytes(&[0x08, 0x01, 0x08, 0x02]).expect("decode");
        assert_eq!(hello.proto_major, 2);
    }

    #[test]
    fn truncated_length_is_an_error() {
        let bytes = [0x1a, 0x05, b'a'];
        assert!(matches!(
            Hello::from_bytes(&bytes),
            Err(ProtoError::Truncated)
        ));
    }

    #[test]
    fn field_zero_and_bad_wire_type_are_errors() {
        assert!(matches!(
            Hello::from_bytes(&[0x00]),
            Err(ProtoError::BadFieldNumber)
        ));
        assert!(matches!(
            Hello::from_bytes(&[0x0f, 0x01]),
            Err(ProtoError::BadWireType(7))
        ));
    }

    #[test]
    fn oversize_control_message_is_rejected() {
        let big = vec![0u8; CONTROL_MAX_BYTES + 1];
        assert!(matches!(
            Hello::from_bytes(&big),
            Err(ProtoError::TooLarge { .. })
        ));
        assert!(check_payload(PayloadKind::VideoFrame, VIDEO_FRAME_MAX_BYTES).is_ok());
        assert!(check_payload(PayloadKind::VideoFrame, VIDEO_FRAME_MAX_BYTES + 1).is_err());
        assert_eq!(CLIPBOARD_DEFAULT_MAX_BYTES, CLIPBOARD_ABSOLUTE_MAX_BYTES);
    }

    #[test]
    fn non_finite_float_is_rejected() {
        let display = DisplayInfo {
            scale: f32::NAN,
            ..DisplayInfo::default()
        };
        assert!(matches!(display.to_bytes(), Err(ProtoError::NonFinite)));
    }

    #[test]
    fn version_policy() {
        assert!(major_compatible(1, 1));
        assert!(major_compatible(2, 1));
        assert!(major_compatible(1, 2));
        assert!(!major_compatible(1, 3));
        assert!(!major_compatible(0, 1));
        assert!(session_supports(PROTO_MAJOR));
        assert!(!session_supports(2));
        assert_eq!(ALPN, "cbdesk/1");
        assert!(!supports_hidden_session());
    }

    #[test]
    fn high_risk_permissions_match_the_plan() {
        assert!(Permission::FileWrite.is_high_risk());
        assert!(Permission::Terminal.is_high_risk());
        assert!(Permission::BlockInput.is_high_risk());
        assert!(Permission::PrivacyMode.is_high_risk());
        assert!(!Permission::View.is_high_risk());
        assert!(!Permission::FileRead.is_high_risk());
        assert!(Permission::Unknown(99).bit().is_none());
        assert_eq!(Permission::View.bit(), Some(1));
    }

    #[test]
    fn catalogs_cover_every_error_and_match_across_languages() {
        let en: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../../../app/i18n/en.json")).expect("en json");
        let tr: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../../../app/i18n/tr.json")).expect("tr json");
        assert_eq!(en.keys().collect::<Vec<_>>(), tr.keys().collect::<Vec<_>>());
        for code in ErrorCode::known() {
            assert!(
                en.contains_key(code.message_key()),
                "{}",
                code.message_key()
            );
            assert!(en.contains_key(code.hint_key()), "{}", code.hint_key());
        }
        assert!(en.contains_key(ErrorCode::Unknown(1).message_key()));
        assert!(en.contains_key(ErrorCode::Unknown(1).hint_key()));
        for key in UI_MESSAGE_KEYS {
            let en_value = en.get(*key).unwrap_or_else(|| panic!("missing {key}"));
            let tr_value = tr.get(*key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(en_value.as_str().is_some_and(|s| !s.is_empty()));
            assert!(tr_value.as_str().is_some_and(|s| !s.is_empty()));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn hello_roundtrip_random(
            major in 0u32..8,
            minor in 0u32..8,
            version in "[a-z0-9.]{0,12}",
            perm in proptest::sample::select(vec![Permission::View, Permission::Input, Permission::Terminal, Permission::Unknown(70)]),
        ) {
            let hello = Hello {
                proto_major: major,
                proto_minor: minor,
                app_version: version,
                platform: Platform::Windows,
                caps: vec![Capability::Ipv6],
                wanted_perms: vec![perm],
                auth_offer: vec![AuthMethod::PakeCode],
                device_name_enc: vec![1, 2, 3],
                unknown: UnknownFields::new(),
            };
            let bytes = hello.to_bytes().expect("encode");
            let back = Hello::from_bytes(&bytes).expect("decode");
            prop_assert_eq!(back, hello);
        }

        #[test]
        fn garbage_does_not_panic(bytes in proptest::collection::vec(any::<u8>(), 0..128)) {
            // Stable stand-in for a fuzz target. cargo-fuzz and OSS-Fuzz
            // are not wired, and this does not claim they ran.
            let _ = Hello::from_bytes(&bytes);
            let _ = InviteTicket::from_bytes(&bytes);
            let _ = Granted::from_bytes(&bytes);
            let _ = VideoFrameHeader::from_bytes(&bytes);
            let _ = Feedback::from_bytes(&bytes);
            let _ = ClipboardText::from_bytes(&bytes);
            let _ = FileChunk::from_bytes(&bytes);
            let _ = InputEvent::from_bytes(&bytes);
            let _ = Control::from_bytes(&bytes);
        }
    }

    fn unknown_len(number: u32, bytes: &[u8]) -> UnknownFields {
        let mut unknown = UnknownFields::new();
        unknown.push(number, FieldValue::Len(bytes.to_vec()));
        unknown
    }
}

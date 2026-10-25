// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! View-side controller.
//!
//! This crate is the part of the product an iOS target can compile: protocol
//! constants and an explicit refusal to host. It does not link screen capture,
//! input injection, a microphone, or the host keystore. A signed TestFlight
//! build needs an Apple Developer account, which this tree does not have.
//! macOS CI runs `cargo check -p cbdesk-controller --target aarch64-apple-ios`
//! when that SDK is installed.

/// Role reported by this crate.
pub const ROLE: &str = "controller";

/// Protocol major this controller speaks.
pub const fn protocol_major() -> u32 {
    cbdesk_proto::PROTO_MAJOR
}

/// This crate does not capture a display or inject input.
pub const fn host_features_linked() -> bool {
    false
}

/// What a controller build is willing to do.
pub const fn view_features() -> &'static [&'static str] {
    &["protocol", "consent-client"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_controller_does_not_host() {
        assert_eq!(ROLE, "controller");
        assert!(!host_features_linked());
        assert!(view_features().contains(&"protocol"));
        assert!(protocol_major() >= 1);
    }
}

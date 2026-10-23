// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Connection attempts that do not depend on a vendor relay network.
//!
//! The [`Transport`] trait returns the concrete [`Conn`] used by the in-memory
//! hub. It is an `async fn` in a trait (stable since Rust 1.75) rather than a
//! trait object, and it does not pull in the `async-trait` crate, so this
//! workspace can keep `forbid(unsafe_code)`. [`QuicEndpoint`] is the direct
//! QUIC adapter. It is not the memory [`Conn`]: iroh's connection type stays
//! inside this crate.

mod addr;
mod backoff;
mod error;
mod memory;
mod path;
mod policy;
mod quic;

pub use addr::{address_rank, rank_candidates, MAX_CANDIDATES};
pub use backoff::{next_delay_ms, BACKOFF_MS, GIVE_UP_MS};
pub use error::NetError;
pub use memory::{Conn, MemoryEndpoint, MemoryHub};
pub use path::{apply, PathEvent, PathState};
pub use policy::{
    check_hints, relay_host, validate_endpoint, EndpointConfig, RelayHint, FORBIDDEN_RELAY_SUFFIXES,
};
pub use quic::{QuicConn, QuicEndpoint};

use cbdesk_crypto::DeviceId;

/// QUIC port chosen for CBDesk. It is outside 21115–21119.
pub const QUIC_PORT: u16 = 22116;

/// Delay between happy-eyeballs candidate attempts.
pub const HAPPY_EYEBALLS_STAGGER_MS: u64 = 250;

/// How long a direct attempt runs before the path moves toward a relay.
pub const DIRECT_TIMEOUT_MS: u64 = 3_000;

/// How long to wait for the relay once a direct attempt has failed.
pub const RELAY_FALLBACK_MS: u64 = 4_000;

/// Something that can move bytes to another device.
///
/// Implementations must run [`check_hints`] before they expose a connection.
pub trait Transport {
    /// Open a connection to `peer`.
    fn dial(
        &self,
        peer: DeviceId,
        hints: &[RelayHint],
    ) -> impl std::future::Future<Output = Result<Conn, NetError>> + Send;

    /// Take the next inbound connection.
    fn accept(&self) -> impl std::future::Future<Output = Result<Conn, NetError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_constants_match_the_milestone_defaults() {
        assert_eq!(QUIC_PORT, 22116);
        assert!(!(21115..=21119).contains(&QUIC_PORT));
        assert_eq!(HAPPY_EYEBALLS_STAGGER_MS, 250);
        assert_eq!(DIRECT_TIMEOUT_MS, 3_000);
        assert_eq!(RELAY_FALLBACK_MS, 4_000);
        assert_eq!(MAX_CANDIDATES, 8);
    }
}

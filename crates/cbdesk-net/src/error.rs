// SPDX-License-Identifier: AGPL-3.0-or-later

//! Failures from dialing, path changes, and relay policy.

use thiserror::Error;

/// A networking operation was refused or could not continue.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NetError {
    /// The endpoint was asked to dial its own device id.
    #[error("an endpoint cannot dial itself")]
    SelfDial,
    /// No in-memory listener is bound for that device.
    #[error("peer is not available")]
    PeerUnavailable,
    /// That device id is already bound on this hub.
    #[error("device id is already bound")]
    AlreadyBound,
    /// The connection or listener is gone.
    #[error("connection closed")]
    Closed,
    /// A non-blocking accept found the listener lock busy.
    #[error("listener is busy")]
    Busy,
    /// `use_n0_defaults` is not allowed.
    #[error("n0 default relays and discovery servers are forbidden")]
    N0DefaultsForbidden,
    /// A relay host is on the forbidden suffix list.
    #[error("relay host `{host}` is forbidden")]
    ForbiddenRelay {
        /// The host that matched a forbidden suffix.
        host: String,
    },
    /// The endpoint was not built with this relay, or it has no relays at all.
    #[error("operator relays are not installed on the direct QUIC endpoint")]
    RelaysNotInstalled,
    /// A relay URL could not be parsed.
    #[error("relay URL `{0}` is not usable")]
    BadRelay(String),
    /// Relay-only mode was asked for without a relay URL.
    #[error("relay-only mode needs a user-configured relay")]
    RelayOnlyNeedsRelay,
    /// Isolated mode was combined with a relay.
    #[error("isolated mode does not use a relay")]
    IsolatedRejectsRelay,
    /// The QUIC stack refused to bind or to move bytes.
    #[error("QUIC endpoint failed: {0}")]
    Quic(String),
    /// No direct address was eligible to dial.
    #[error("no dialable direct address")]
    NoCandidate,
    /// The control stream did not carry one `cbdesk/1` frame.
    #[error("control frame was rejected")]
    BadControl,
    /// The first stream was not a SPAKE2 pairing handshake.
    #[error("pairing is required")]
    PairingRequired,
    /// The SPAKE2 confirmation tags did not match.
    #[error("pairing failed")]
    PairingFailed,
    /// A video or input frame was the wrong size or could not be parsed.
    #[error("media frame was rejected")]
    BadMedia,
    /// The path state machine rejected this event.
    #[error("path cannot apply {event} while {state}")]
    InvalidTransition {
        /// Current state name.
        state: &'static str,
        /// Event name.
        event: &'static str,
    },
}

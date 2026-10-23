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
    /// The direct endpoint does not install operator relays.
    #[error("operator relays are not installed on the direct QUIC endpoint")]
    RelaysNotInstalled,
    /// The QUIC stack refused to bind or to move bytes.
    #[error("QUIC endpoint failed: {0}")]
    Quic(String),
    /// No direct address was eligible to dial.
    #[error("no dialable direct address")]
    NoCandidate,
    /// The control stream did not carry one `cbdesk/1` frame.
    #[error("control frame was rejected")]
    BadControl,
    /// The path state machine rejected this event.
    #[error("path cannot apply {event} while {state}")]
    InvalidTransition {
        /// Current state name.
        state: &'static str,
        /// Event name.
        event: &'static str,
    },
}

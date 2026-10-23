// SPDX-License-Identifier: AGPL-3.0-or-later

//! Direct-versus-relay path state.
//!
//! Invalid transitions return an error and the caller keeps the previous
//! state. This module does not open sockets.

use crate::error::NetError;

/// Where the session's packets are going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathState {
    /// Nothing has been tried.
    Idle,
    /// Direct candidates are in flight.
    TryingDirect,
    /// A direct path is up.
    Direct,
    /// A relay is being contacted.
    TryingRelay,
    /// A relay path is up.
    Relay,
    /// A relay path is up and a direct upgrade is in flight.
    Upgrading,
    /// The attempt was abandoned.
    Failed,
}

/// Something the path machine can observe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathEvent {
    /// Begin or restart a direct attempt.
    StartDirect,
    /// A direct candidate connected.
    DirectEstablished,
    /// The direct attempt passed its timeout.
    DirectTimedOut,
    /// The relay connected.
    RelayEstablished,
    /// A hole-punch upgraded a relay path.
    HolepunchEstablished,
    /// The current path disappeared.
    PathLost,
    /// The user or the session gave up.
    Abandon,
}

impl PathState {
    /// Stable name used in errors.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::TryingDirect => "trying-direct",
            Self::Direct => "direct",
            Self::TryingRelay => "trying-relay",
            Self::Relay => "relay",
            Self::Upgrading => "upgrading",
            Self::Failed => "failed",
        }
    }
}

impl PathEvent {
    /// Stable name used in errors.
    pub const fn name(self) -> &'static str {
        match self {
            Self::StartDirect => "start-direct",
            Self::DirectEstablished => "direct-established",
            Self::DirectTimedOut => "direct-timed-out",
            Self::RelayEstablished => "relay-established",
            Self::HolepunchEstablished => "holepunch-established",
            Self::PathLost => "path-lost",
            Self::Abandon => "abandon",
        }
    }
}

/// Apply `event` to `state`.
pub fn apply(state: PathState, event: PathEvent) -> Result<PathState, NetError> {
    let next = match (state, event) {
        (PathState::Idle, PathEvent::StartDirect) => PathState::TryingDirect,
        (PathState::TryingDirect, PathEvent::DirectEstablished) => PathState::Direct,
        (PathState::TryingDirect, PathEvent::DirectTimedOut) => PathState::TryingRelay,
        (PathState::TryingRelay, PathEvent::RelayEstablished) => PathState::Relay,
        (PathState::Relay, PathEvent::StartDirect) => PathState::Upgrading,
        (PathState::Upgrading, PathEvent::HolepunchEstablished) => PathState::Direct,
        (PathState::Upgrading, PathEvent::DirectTimedOut) => PathState::Relay,
        (PathState::Direct, PathEvent::PathLost) => PathState::TryingDirect,
        (PathState::Relay | PathState::Upgrading, PathEvent::PathLost) => PathState::TryingRelay,
        (
            PathState::TryingDirect
            | PathState::Direct
            | PathState::TryingRelay
            | PathState::Relay
            | PathState::Upgrading,
            PathEvent::Abandon,
        ) => PathState::Failed,
        (PathState::Failed, PathEvent::StartDirect) => PathState::TryingDirect,
        _ => {
            return Err(NetError::InvalidTransition {
                state: state.name(),
                event: event.name(),
            });
        }
    };
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_transitions_match_the_path_table() {
        let cases = [
            (
                PathState::Idle,
                PathEvent::StartDirect,
                PathState::TryingDirect,
            ),
            (
                PathState::TryingDirect,
                PathEvent::DirectEstablished,
                PathState::Direct,
            ),
            (
                PathState::TryingDirect,
                PathEvent::DirectTimedOut,
                PathState::TryingRelay,
            ),
            (
                PathState::TryingRelay,
                PathEvent::RelayEstablished,
                PathState::Relay,
            ),
            (
                PathState::Relay,
                PathEvent::StartDirect,
                PathState::Upgrading,
            ),
            (
                PathState::Upgrading,
                PathEvent::HolepunchEstablished,
                PathState::Direct,
            ),
            (
                PathState::Upgrading,
                PathEvent::DirectTimedOut,
                PathState::Relay,
            ),
            (
                PathState::Direct,
                PathEvent::PathLost,
                PathState::TryingDirect,
            ),
            (
                PathState::Relay,
                PathEvent::PathLost,
                PathState::TryingRelay,
            ),
            (
                PathState::Upgrading,
                PathEvent::PathLost,
                PathState::TryingRelay,
            ),
            (
                PathState::TryingDirect,
                PathEvent::Abandon,
                PathState::Failed,
            ),
            (PathState::Direct, PathEvent::Abandon, PathState::Failed),
            (
                PathState::TryingRelay,
                PathEvent::Abandon,
                PathState::Failed,
            ),
            (PathState::Relay, PathEvent::Abandon, PathState::Failed),
            (PathState::Upgrading, PathEvent::Abandon, PathState::Failed),
            (
                PathState::Failed,
                PathEvent::StartDirect,
                PathState::TryingDirect,
            ),
        ];
        for (state, event, expected) in cases {
            assert_eq!(apply(state, event).expect("legal"), expected);
        }
    }

    #[test]
    fn illegal_transitions_leave_the_caller_with_an_error() {
        let illegal = [
            (PathState::Idle, PathEvent::Abandon),
            (PathState::Idle, PathEvent::DirectEstablished),
            (PathState::Failed, PathEvent::Abandon),
            (PathState::Direct, PathEvent::RelayEstablished),
            (PathState::TryingRelay, PathEvent::DirectEstablished),
        ];
        for (state, event) in illegal {
            assert!(apply(state, event).is_err());
        }
    }
}

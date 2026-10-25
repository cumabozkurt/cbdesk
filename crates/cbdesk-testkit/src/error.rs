// SPDX-License-Identifier: AGPL-3.0-or-later

//! Errors from the fake DHT. Frame errors live in `cbdesk-capture`.

use thiserror::Error;

use cbdesk_discovery::DiscoveryError;

/// A test double rejected its input.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestkitError {
    /// The fake DHT could not sign or verify a record.
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

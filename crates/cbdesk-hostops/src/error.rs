// SPDX-License-Identifier: AGPL-3.0-or-later

//! Failures from a host operating-system effect.

use thiserror::Error;

/// The effect was refused or the operating system rejected it.
#[derive(Debug, Error)]
pub enum OsError {
    /// The permit was for a different effect.
    #[error("the permit does not allow this effect")]
    WrongPermit,
    /// This platform cannot do the effect.
    #[error("operating-system effect is unavailable: {0}")]
    Unavailable(String),
    /// The operating system returned an error.
    #[error("operating-system effect failed: {0}")]
    Failed(String),
    /// A tunnel destination was not a loopback address.
    #[error("tunnel destination was refused: {0}")]
    Refused(String),
}

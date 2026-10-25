// SPDX-License-Identifier: AGPL-3.0-or-later

//! Injection failures.

use thiserror::Error;

/// The event could not be applied.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InputError {
    /// This machine has no input backend, or the desktop refused the connection.
    #[error("input injection is unavailable: {0}")]
    Unavailable(String),
    /// The desktop rejected the event.
    #[error("input injection failed: {0}")]
    Failed(String),
    /// The keycode is not one this build will inject.
    #[error("keycode {0} is not injectable")]
    UnsupportedKey(u32),
}

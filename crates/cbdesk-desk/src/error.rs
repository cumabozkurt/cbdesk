// SPDX-License-Identifier: AGPL-3.0-or-later

//! Failures the desktop shell can show.

use thiserror::Error;

/// Why a desktop action stopped.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DeskError {
    /// A short code was typed without a ticket.
    ///
    /// The words can confirm pairing. They do not locate a computer.
    #[error("a short code does not locate a computer")]
    ShortCodeNeedsTicket,
    /// This device id is on the local block list.
    #[error("that device is blocked")]
    Blocked,
    /// Accept, reject, or block was pressed with no prompt.
    #[error("there is no consent prompt")]
    NoPrompt,
    /// The seed file is readable by someone other than the owner.
    #[error("the identity file is readable by other users")]
    LooseSeed,
    /// The clock or the state lock was poisoned, or another local failure.
    #[error("{0}")]
    Message(String),
}

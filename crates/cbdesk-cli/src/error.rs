// SPDX-License-Identifier: AGPL-3.0-or-later

//! CLI failures. Details stay off the ticket and off the seed.

use thiserror::Error;

/// The command could not finish.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct CliError(String);

impl CliError {
    /// An error whose text is safe to print.
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        Self::new(err.to_string())
    }
}

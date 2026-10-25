// SPDX-License-Identifier: AGPL-3.0-or-later

//! A debug wrapper that refuses to print a secret.

use std::fmt;

/// A value whose `Debug` output is always `[REDACTED]`.
///
/// Use this for passwords, seeds, PAKE hints, and clipboard previews before
/// they get near a log. Reading the value requires [`Redacted::expose`].
#[derive(Clone, Eq, PartialEq)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    /// Wrap `value`.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the secret. Callers should not log the result.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consume the wrapper and return the secret.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

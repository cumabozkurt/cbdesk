// SPDX-License-Identifier: AGPL-3.0-or-later

//! Session and idle deadlines.
//!
//! Times are milliseconds from an arbitrary clock the caller supplies. This
//! module does not read the wall clock and does not close a socket.

use crate::SessionError;

/// Limits for one granted session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionDeadline {
    started_ms: u64,
    limit_ms: u64,
    idle_limit_ms: u64,
    last_activity_ms: u64,
}

impl SessionDeadline {
    /// `limit_ms` and `idle_limit_ms` must be non-zero.
    pub fn new(started_ms: u64, limit_ms: u64, idle_limit_ms: u64) -> Result<Self, SessionError> {
        if limit_ms == 0 || idle_limit_ms == 0 {
            return Err(SessionError::BadConfig);
        }
        Ok(Self {
            started_ms,
            limit_ms,
            idle_limit_ms,
            last_activity_ms: started_ms,
        })
    }

    /// Remember `now_ms` as the latest activity, when it is not earlier than the start.
    pub fn activity(&mut self, now_ms: u64) -> Result<(), SessionError> {
        if now_ms < self.started_ms {
            return Err(SessionError::BadConfig);
        }
        self.last_activity_ms = now_ms;
        Ok(())
    }

    /// Whether `now_ms` is past the session limit or the idle limit.
    pub fn expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.started_ms) >= self.limit_ms
            || now_ms.saturating_sub(self.last_activity_ms) >= self.idle_limit_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_and_absolute_limits_expire() {
        let mut deadline = SessionDeadline::new(1_000, 10_000, 2_000).expect("deadline");
        assert!(!deadline.expired(2_500));
        deadline.activity(2_500).expect("activity");
        assert!(!deadline.expired(4_000));
        assert!(deadline.expired(4_500));
        assert!(deadline.expired(11_000));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reconnect delays.
//!
//! The give-up check happens before a delay is chosen. Once ten minutes have
//! already elapsed, no further attempt is scheduled.

/// Delays for attempts 0 through 5, in milliseconds. Later attempts stay at 15 s.
pub const BACKOFF_MS: [u64; 6] = [500, 1_000, 2_000, 4_000, 8_000, 15_000];

/// Stop scheduling when this much time has already passed.
pub const GIVE_UP_MS: u64 = 10 * 60 * 1_000;

/// Delay before the next attempt, or `None` when the caller must stop.
pub fn next_delay_ms(attempt: u32, elapsed_ms: u64) -> Option<u64> {
    if elapsed_ms >= GIVE_UP_MS {
        return None;
    }
    let index = usize::try_from(attempt).unwrap_or(BACKOFF_MS.len());
    Some(BACKOFF_MS.get(index).copied().unwrap_or(15_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_follow_the_ladder_and_then_stick() {
        let expected = [500, 1_000, 2_000, 4_000, 8_000, 15_000, 15_000];
        for (attempt, delay) in expected.iter().copied().enumerate() {
            let attempt = u32::try_from(attempt).expect("attempt");
            assert_eq!(next_delay_ms(attempt, 0), Some(delay));
        }
    }

    #[test]
    fn give_up_is_checked_before_a_delay_is_chosen() {
        assert_eq!(next_delay_ms(0, GIVE_UP_MS - 1), Some(500));
        assert_eq!(next_delay_ms(0, GIVE_UP_MS), None);
        assert_eq!(next_delay_ms(3, GIVE_UP_MS + 1), None);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authentication and consent-prompt limits.
//!
//! Failures are counted per device id and per network prefix (IPv4 /24,
//! IPv6 /56). An IPv4-mapped IPv6 address uses the IPv4 prefix. The fifth
//! failure locks for 60 seconds. Each further failure doubles that lock, up
//! to one hour. A success clears only the device counter.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use cbdesk_crypto::DeviceId;

use crate::error::SessionError;

const FAILURES_BEFORE_LOCK: u32 = 5;
const INITIAL_LOCK_MS: u64 = 60_000;
const MAX_LOCK_MS: u64 = 3_600_000;
const MAX_PENDING_HANDSHAKES: u32 = 16;
const PROMPT_GAP_MS: u64 = 60_000;
const REJECTS_BEFORE_SILENCE: u32 = 3;
const SILENCE_MS: u64 = 24 * 60 * 60 * 1_000;

/// IPv4 /24 or IPv6 /56 prefix used by the shared failure counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NetPrefix {
    /// First three octets.
    V4([u8; 3]),
    /// First seven octets (56 bits).
    V6([u8; 7]),
}

/// Prefix of `addr`. Mapped IPv4 is treated as IPv4.
pub fn net_prefix(addr: IpAddr) -> NetPrefix {
    match addr {
        IpAddr::V4(ip) => prefix_v4(ip),
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or_else(|| prefix_v6(ip), prefix_v4),
    }
}

fn prefix_v4(ip: Ipv4Addr) -> NetPrefix {
    let octets = ip.octets();
    NetPrefix::V4([octets[0], octets[1], octets[2]])
}

fn prefix_v6(ip: Ipv6Addr) -> NetPrefix {
    let octets = ip.octets();
    NetPrefix::V6([
        octets[0], octets[1], octets[2], octets[3], octets[4], octets[5], octets[6],
    ])
}

#[derive(Clone, Debug, Default)]
struct FailState {
    failures: u32,
    lock_ms: u64,
    locked_until_ms: u64,
}

impl FailState {
    fn locked(&self, now_ms: u64) -> bool {
        self.locked_until_ms > now_ms
    }

    fn note(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= FAILURES_BEFORE_LOCK {
            self.lock_ms = if self.lock_ms == 0 {
                INITIAL_LOCK_MS
            } else {
                self.lock_ms.saturating_mul(2).min(MAX_LOCK_MS)
            };
            self.locked_until_ms = now_ms.saturating_add(self.lock_ms);
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug, Default)]
struct PromptState {
    last_prompt_ms: Option<u64>,
    rejects: u32,
    silent_until_ms: u64,
}

/// Counters for failed authentication and consent prompts.
#[derive(Clone, Debug, Default)]
pub struct RateLimiter {
    peers: HashMap<DeviceId, FailState>,
    prefixes: HashMap<NetPrefix, FailState>,
    pending: u32,
    prompts: HashMap<DeviceId, PromptState>,
}

impl RateLimiter {
    /// Empty counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Count a failed attempt against the peer and its prefix.
    pub fn record_failure(&mut self, peer: DeviceId, source: IpAddr, now_ms: u64) {
        self.peers.entry(peer).or_default().note(now_ms);
        self.prefixes
            .entry(net_prefix(source))
            .or_default()
            .note(now_ms);
    }

    /// Clear the peer counter. The prefix counter stays.
    pub fn record_success(&mut self, peer: &DeviceId) {
        if let Some(state) = self.peers.get_mut(peer) {
            state.reset();
        }
    }

    /// True when the peer or its prefix is inside a lock window.
    pub fn is_limited(&self, peer: &DeviceId, source: IpAddr, now_ms: u64) -> bool {
        let peer_locked = self
            .peers
            .get(peer)
            .is_some_and(|state| state.locked(now_ms));
        let prefix_locked = self
            .prefixes
            .get(&net_prefix(source))
            .is_some_and(|state| state.locked(now_ms));
        peer_locked || prefix_locked
    }

    /// Reserve a handshake slot. At most 16 may be outstanding.
    pub fn begin_handshake(&mut self) -> Result<(), SessionError> {
        if self.pending >= MAX_PENDING_HANDSHAKES {
            return Err(SessionError::TooManyHandshakes);
        }
        self.pending += 1;
        Ok(())
    }

    /// Release a handshake slot.
    pub fn end_handshake(&mut self) {
        self.pending = self.pending.saturating_sub(1);
    }

    /// Allow at most one consent prompt per peer per minute.
    pub fn begin_prompt(&mut self, peer: DeviceId, now_ms: u64) -> Result<(), SessionError> {
        let state = self.prompts.entry(peer).or_default();
        if now_ms < state.silent_until_ms {
            return Err(SessionError::PromptSilenced);
        }
        if let Some(last) = state.last_prompt_ms {
            if now_ms.saturating_sub(last) < PROMPT_GAP_MS {
                return Err(SessionError::PromptTooSoon);
            }
        }
        state.last_prompt_ms = Some(now_ms);
        Ok(())
    }

    /// Count a rejection. The third one silences prompts for 24 hours.
    pub fn record_reject(&mut self, peer: DeviceId, now_ms: u64) {
        let state = self.prompts.entry(peer).or_default();
        if now_ms < state.silent_until_ms {
            return;
        }
        state.rejects = state.rejects.saturating_add(1);
        if state.rejects >= REJECTS_BEFORE_SILENCE {
            state.rejects = 0;
            state.silent_until_ms = now_ms.saturating_add(SILENCE_MS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_crypto::Identity;

    fn peer(seed: u8) -> DeviceId {
        Identity::from_seed(&[seed; 32]).expect("id").device_id()
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("ip")
    }

    #[test]
    fn fifth_failure_locks_and_success_does_not_clear_the_prefix() {
        let mut limits = RateLimiter::new();
        let peer_a = peer(1);
        let source = ip("203.0.113.10");
        for _ in 0..4 {
            limits.record_failure(peer_a, source, 0);
        }
        assert!(!limits.is_limited(&peer_a, source, 0));
        limits.record_failure(peer_a, source, 1_000);
        assert!(limits.is_limited(&peer_a, source, 1_000));
        assert!(!limits.is_limited(&peer_a, source, 1_000 + INITIAL_LOCK_MS));
        limits.record_failure(peer_a, source, 1_000 + INITIAL_LOCK_MS);
        assert!(limits.is_limited(&peer_a, source, 1_000 + INITIAL_LOCK_MS + 120_000 - 1));
        assert!(!limits.is_limited(&peer_a, source, 1_000 + INITIAL_LOCK_MS + 120_000));

        let peer_b = peer(2);
        limits.record_success(&peer_a);
        assert!(limits.is_limited(&peer_a, source, 2_000));
        assert!(limits.is_limited(&peer_b, source, 2_000));
        let other = ip("198.51.100.4");
        for _ in 0..4 {
            limits.record_failure(peer_a, other, 3_000);
        }
        assert!(!limits.is_limited(&peer_a, other, 3_000));
    }

    #[test]
    fn prefixes_group_ipv4_ipv6_and_mapped_addresses() {
        let v4_a = ip("192.0.2.1");
        let v4_b = ip("192.0.2.200");
        let v4_other = ip("192.0.3.1");
        assert_eq!(net_prefix(v4_a), net_prefix(v4_b));
        assert_ne!(net_prefix(v4_a), net_prefix(v4_other));
        let v6_a = ip("2001:db8:1:20::1");
        let v6_b = ip("2001:db8:1:ff::1");
        let v6_other = ip("2001:db8:1:100::1");
        assert_eq!(net_prefix(v6_a), net_prefix(v6_b));
        assert_ne!(net_prefix(v6_a), net_prefix(v6_other));
        let mapped = IpAddr::V6(Ipv4Addr::new(192, 0, 2, 9).to_ipv6_mapped());
        assert_eq!(net_prefix(mapped), net_prefix(v4_a));
    }

    #[test]
    fn handshakes_stop_at_sixteen() {
        let mut limits = RateLimiter::new();
        for _ in 0..16 {
            limits.begin_handshake().expect("slot");
        }
        assert_eq!(
            limits.begin_handshake(),
            Err(SessionError::TooManyHandshakes)
        );
        limits.end_handshake();
        limits.begin_handshake().expect("slot again");
    }

    #[test]
    fn prompts_are_one_per_minute_and_three_rejects_go_quiet() {
        let mut limits = RateLimiter::new();
        let peer = peer(9);
        limits.begin_prompt(peer, 0).expect("first");
        assert_eq!(
            limits.begin_prompt(peer, 59_999),
            Err(SessionError::PromptTooSoon)
        );
        limits.begin_prompt(peer, 60_000).expect("next minute");
        limits.record_reject(peer, 60_000);
        limits.record_reject(peer, 60_000);
        limits.begin_prompt(peer, 120_000).expect("still allowed");
        limits.record_reject(peer, 120_000);
        assert_eq!(
            limits.begin_prompt(peer, 180_000),
            Err(SessionError::PromptSilenced)
        );
        let after = 120_000 + SILENCE_MS;
        limits.begin_prompt(peer, after).expect("silence ended");
        limits.record_reject(peer, after);
        limits
            .begin_prompt(peer, after + PROMPT_GAP_MS)
            .expect("counter reset");
    }
}

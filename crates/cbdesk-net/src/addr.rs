// SPDX-License-Identifier: AGPL-3.0-or-later

//! Candidate ordering for a direct connection attempt.
//!
//! Lower ranks are tried first. Unspecified, multicast, and IPv4 broadcast
//! addresses are dropped. IPv4-mapped IPv6 addresses are ranked as IPv4.
//! Ranking is a stable sort, then truncated to [`MAX_CANDIDATES`].

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use cbdesk_proto::MAX_DIRECT_ADDRS;

/// How many direct candidates a dial keeps.
pub const MAX_CANDIDATES: usize = MAX_DIRECT_ADDRS;

const RANK_V6_GLOBAL: u8 = 0;
const RANK_V4_GLOBAL: u8 = 1;
const RANK_V6_ULA: u8 = 2;
const RANK_V4_PRIVATE: u8 = 3;
const RANK_V6_LINK_LOCAL: u8 = 4;
const RANK_V4_LINK_LOCAL: u8 = 5;
const RANK_V6_LOOPBACK: u8 = 6;
const RANK_V4_LOOPBACK: u8 = 7;

/// Rank of `addr`, if it is eligible to be dialed.
///
/// `None` means the address is dropped. Ranks follow the order in the module
/// docs: global IPv6, global IPv4, IPv6 ULA, IPv4 private (including CGNAT
/// `100.64.0.0/10`), IPv6 link-local, IPv4 link-local, IPv6 loopback, IPv4
/// loopback.
pub fn address_rank(addr: SocketAddr) -> Option<u8> {
    match addr.ip() {
        IpAddr::V4(ip) => rank_v4(ip),
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return rank_v4(mapped);
            }
            rank_v6(ip)
        }
    }
}

/// Stable-sort `addrs` by [`address_rank`] and keep at most [`MAX_CANDIDATES`].
pub fn rank_candidates(addrs: &[SocketAddr]) -> Vec<SocketAddr> {
    let mut ranked: Vec<(u8, SocketAddr)> = addrs
        .iter()
        .copied()
        .filter_map(|addr| address_rank(addr).map(|rank| (rank, addr)))
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked
        .into_iter()
        .map(|(_, addr)| addr)
        .take(MAX_CANDIDATES)
        .collect()
}

fn rank_v4(ip: Ipv4Addr) -> Option<u8> {
    if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() {
        return None;
    }
    let [first, second, _, _] = ip.octets();
    if first == 127 {
        return Some(RANK_V4_LOOPBACK);
    }
    if first == 169 && second == 254 {
        return Some(RANK_V4_LINK_LOCAL);
    }
    if is_v4_private(first, second) {
        return Some(RANK_V4_PRIVATE);
    }
    Some(RANK_V4_GLOBAL)
}

fn is_v4_private(first: u8, second: u8) -> bool {
    first == 10
        || (first == 172 && (16..=31).contains(&second))
        || (first == 192 && second == 168)
        || (first == 100 && (64..=127).contains(&second))
}

fn rank_v6(ip: Ipv6Addr) -> Option<u8> {
    if ip.is_unspecified() || ip.is_multicast() {
        return None;
    }
    if ip.is_loopback() {
        return Some(RANK_V6_LOOPBACK);
    }
    let first = ip.segments()[0];
    if first & 0xffc0 == 0xfe80 {
        return Some(RANK_V6_LINK_LOCAL);
    }
    if first & 0xfe00 == 0xfc00 {
        return Some(RANK_V6_ULA);
    }
    Some(RANK_V6_GLOBAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(text: &str) -> SocketAddr {
        text.parse().expect("socket addr")
    }

    #[test]
    fn candidates_follow_rank_and_keep_input_order_inside_a_rank() {
        let input = [
            addr("127.0.0.1:22116"),
            addr("[::1]:22116"),
            addr("169.254.1.1:22116"),
            addr("[fe80::1]:22116"),
            addr("100.64.1.1:22116"),
            addr("10.0.0.1:22116"),
            addr("[fc00::1]:22116"),
            addr("203.0.113.5:22116"),
            addr("[2001:db8::1]:22116"),
        ];
        let ranked = rank_candidates(&input);
        let expected = [
            addr("[2001:db8::1]:22116"),
            addr("203.0.113.5:22116"),
            addr("[fc00::1]:22116"),
            addr("100.64.1.1:22116"),
            addr("10.0.0.1:22116"),
            addr("[fe80::1]:22116"),
            addr("169.254.1.1:22116"),
            addr("[::1]:22116"),
        ];
        assert_eq!(ranked, expected);
    }

    #[test]
    fn unsuitable_addresses_are_dropped_and_the_list_stops_at_eight() {
        let mut input = vec![
            addr("0.0.0.0:22116"),
            addr("255.255.255.255:22116"),
            addr("224.0.0.1:22116"),
            addr("[::]:22116"),
            addr("[ff02::1]:22116"),
        ];
        for host in 1..=9 {
            input.push(addr(&format!("203.0.113.{host}:22116")));
        }
        let ranked = rank_candidates(&input);
        assert_eq!(ranked.len(), MAX_CANDIDATES);
        assert_eq!(ranked[0], addr("203.0.113.1:22116"));
        assert_eq!(ranked[7], addr("203.0.113.8:22116"));
    }

    #[test]
    fn mapped_ipv4_uses_the_ipv4_rank() {
        let mapped = addr("[::ffff:192.168.1.9]:22116");
        let plain = addr("192.168.1.9:22116");
        assert_eq!(address_rank(mapped), address_rank(plain));
        assert_eq!(address_rank(mapped), Some(RANK_V4_PRIVATE));
    }
}

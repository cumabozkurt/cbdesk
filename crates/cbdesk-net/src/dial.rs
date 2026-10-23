// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dial a verified invite ticket.
//!
//! Direct addresses are tried first. Relay URLs are used only when this
//! endpoint was configured with that same URL and the direct path did not
//! connect. A relay that appears only on the ticket is ignored, so a ticket
//! cannot steer the dialer onto a relay the operator did not choose. A
//! verified ticket still does not authorize a session.

use std::net::SocketAddr;
use std::time::Duration;

use cbdesk_discovery::{verify_ticket, ReplayCache};
use cbdesk_proto::InviteTicket;
use thiserror::Error;
use tokio::task::JoinSet;

use crate::addr::rank_candidates;
use crate::error::NetError;
use crate::quic::QuicEndpoint;
use crate::{DIRECT_TIMEOUT_MS, HAPPY_EYEBALLS_STAGGER_MS, RELAY_FALLBACK_MS};

/// A ticket could not be turned into a direct QUIC connection.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DialError {
    /// The ticket failed verification, so nothing was dialed.
    #[error(transparent)]
    Discovery(#[from] cbdesk_discovery::DiscoveryError),
    /// The QUIC dial failed, or no address was usable.
    #[error(transparent)]
    Net(#[from] NetError),
    /// A direct address was not `ip:port` or `[ipv6]:port`, or it used port 0.
    #[error("direct address `{0}` is not a socket address")]
    BadAddress(String),
}

/// Parse and rank `ticket.direct_addrs`. Relay URLs are ignored.
pub fn direct_candidates(ticket: &InviteTicket) -> Result<Vec<SocketAddr>, DialError> {
    let mut parsed = Vec::with_capacity(ticket.direct_addrs.len());
    for text in &ticket.direct_addrs {
        let addr = parse_direct(text)?;
        parsed.push(addr);
    }
    let ranked = rank_candidates(&parsed);
    if ranked.is_empty() {
        return Err(DialError::Net(NetError::NoCandidate));
    }
    Ok(ranked)
}

/// Verify `ticket`, then dial a direct address or an allowed user relay.
///
/// Verification consumes the replay cache before the dial. An expired or
/// tampered ticket returns here without a connection. The returned connection
/// is not a grant. Relay attempts use [`RELAY_FALLBACK_MS`] each, and only
/// URLs [`QuicEndpoint::allows_relay`] already accepts.
pub async fn dial_verified_ticket(
    endpoint: &QuicEndpoint,
    ticket: &InviteTicket,
    now_secs: u64,
    replay: &mut ReplayCache,
) -> Result<crate::quic::QuicConn, DialError> {
    let device = verify_ticket(ticket, now_secs, replay)?;
    let peer = device.as_bytes();
    let addrs = match direct_candidates(ticket) {
        Ok(addrs) => addrs,
        Err(DialError::Net(NetError::NoCandidate)) => Vec::new(),
        Err(error) => return Err(error),
    };
    let mut direct_error = None;
    if !addrs.is_empty() {
        match connect_candidates(endpoint, peer, &addrs).await {
            Ok(conn) => return Ok(conn),
            Err(error) => direct_error = Some(error),
        }
    }
    let relays = allowed_relays(endpoint, ticket);
    if relays.is_empty() {
        return Err(DialError::Net(
            direct_error.unwrap_or(NetError::NoCandidate),
        ));
    }
    connect_relays(endpoint, peer, &relays)
        .await
        .map_err(DialError::Net)
}

fn allowed_relays(endpoint: &QuicEndpoint, ticket: &InviteTicket) -> Vec<String> {
    ticket
        .relay_urls
        .iter()
        .filter(|url| endpoint.allows_relay(url))
        .cloned()
        .collect()
}

async fn connect_relays(
    endpoint: &QuicEndpoint,
    peer: &[u8; 32],
    relays: &[String],
) -> Result<crate::quic::QuicConn, NetError> {
    if relays.is_empty() {
        return Err(NetError::NoCandidate);
    }
    let mut last = NetError::NoCandidate;
    for url in relays {
        let attempt = endpoint.connect_path(peer, None, Some(url));
        match tokio::time::timeout(Duration::from_millis(RELAY_FALLBACK_MS), attempt).await {
            Ok(Ok(conn)) => return Ok(conn),
            Ok(Err(error)) => last = error,
            Err(_) => last = NetError::Quic("relay attempt timed out".to_string()),
        }
    }
    Err(last)
}

/// Dial `addrs` in order, starting the next candidate after 250 ms or on failure.
///
/// The whole set gives up at [`DIRECT_TIMEOUT_MS`]. The first success is
/// returned and the other attempts are aborted.
pub async fn connect_candidates(
    endpoint: &QuicEndpoint,
    peer: &[u8; 32],
    addrs: &[SocketAddr],
) -> Result<crate::quic::QuicConn, NetError> {
    if addrs.is_empty() {
        return Err(NetError::NoCandidate);
    }
    let mut inflight = JoinSet::new();
    let mut next = 0_usize;
    let mut last = NetError::NoCandidate;
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_millis(DIRECT_TIMEOUT_MS);
    let mut stagger = started;
    loop {
        spawn_due(
            &mut inflight,
            endpoint,
            peer,
            addrs,
            &mut next,
            &mut stagger,
        );
        if inflight.is_empty() {
            return Err(last);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(last);
        }
        let wait = wait_budget(deadline, stagger, next < addrs.len());
        tokio::select! {
            joined = inflight.join_next() => {
                match joined {
                    Some(Ok(Ok(conn))) => return Ok(conn),
                    Some(Ok(Err(error))) => {
                        last = error;
                        stagger = tokio::time::Instant::now();
                    }
                    Some(Err(_)) => {
                        last = NetError::Quic("dial task failed".to_string());
                        stagger = tokio::time::Instant::now();
                    }
                    None => return Err(last),
                }
            }
            () = tokio::time::sleep(wait) => {}
        }
    }
}

fn parse_direct(text: &str) -> Result<SocketAddr, DialError> {
    let addr: SocketAddr = text
        .parse()
        .map_err(|_| DialError::BadAddress(text.to_string()))?;
    if addr.port() == 0 {
        return Err(DialError::BadAddress(text.to_string()));
    }
    Ok(addr)
}

fn spawn_due(
    inflight: &mut JoinSet<Result<crate::quic::QuicConn, NetError>>,
    endpoint: &QuicEndpoint,
    peer: &[u8; 32],
    addrs: &[SocketAddr],
    next: &mut usize,
    stagger: &mut tokio::time::Instant,
) {
    if *next < addrs.len() && tokio::time::Instant::now() >= *stagger {
        let endpoint = endpoint.clone();
        let peer = *peer;
        let addr = addrs[*next];
        inflight.spawn(async move { endpoint.connect(&peer, addr).await });
        *next += 1;
        *stagger = tokio::time::Instant::now() + Duration::from_millis(HAPPY_EYEBALLS_STAGGER_MS);
    }
}

fn wait_budget(
    deadline: tokio::time::Instant,
    stagger: tokio::time::Instant,
    more: bool,
) -> Duration {
    let now = tokio::time::Instant::now();
    let until_deadline = deadline.saturating_duration_since(now);
    if !more {
        return until_deadline;
    }
    stagger.saturating_duration_since(now).min(until_deadline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_prefer_global_addresses_and_ignore_relays() {
        let ticket = InviteTicket {
            direct_addrs: vec![
                "127.0.0.1:22116".to_string(),
                "192.0.2.1:22116".to_string(),
                "0.0.0.0:22116".to_string(),
            ],
            relay_urls: vec!["https://foo.iroh.link/relay".to_string()],
            ..InviteTicket::default()
        };
        let addrs = direct_candidates(&ticket).expect("addrs");
        assert_eq!(addrs.len(), 2);
        assert_eq!(addrs[0], "192.0.2.1:22116".parse().expect("global"));
        assert_eq!(addrs[1], "127.0.0.1:22116".parse().expect("loop"));
        assert!(matches!(
            direct_candidates(&InviteTicket::default()),
            Err(DialError::Net(NetError::NoCandidate))
        ));
        let portless = InviteTicket {
            direct_addrs: vec!["127.0.0.1:0".to_string()],
            ..InviteTicket::default()
        };
        assert!(matches!(
            direct_candidates(&portless),
            Err(DialError::BadAddress(_))
        ));
    }
}

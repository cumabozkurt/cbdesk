// SPDX-License-Identifier: AGPL-3.0-or-later

//! LAN announce and browse for the CLI.
//!
//! `--lan` is the operator opting in. The library still sends nothing when
//! the caller passes a public network profile. This command passes private,
//! because it does not read the OS network category.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use cbdesk_crypto::{DeviceId, Identity};
use cbdesk_discovery::{Incoming, LanPresence, LanPresenceConfig, NetworkProfile, MDNS_PORT};
use cbdesk_net::QuicEndpoint;

use crate::error::CliError;

/// Join the mDNS group and announce until the process ends.
pub async fn start_announce(endpoint: &QuicEndpoint, identity: &Identity) -> Result<(), CliError> {
    let port = endpoint
        .local_addr()
        .map_err(|err| CliError::new(err.to_string()))?
        .port();
    let presence = LanPresence::bind_multicast(config(identity.device_id(), port))
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    let bound = presence
        .mdns_local()
        .map_err(|err| CliError::new(err.to_string()))?;
    if bound.port() == MDNS_PORT {
        eprintln!("lan mdns 5353");
    } else {
        eprintln!(
            "lan mdns ephemeral {}; sending to 5353, not receiving on it",
            bound.port()
        );
    }
    tokio::spawn(async move {
        loop {
            if presence.announces() {
                let _ = presence.announce().await;
            }
            match presence.recv(Duration::from_secs(3)).await {
                Ok(Some(Incoming::BrowseQuery { .. })) => {
                    let _ = presence.announce_mdns().await;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
    Ok(())
}

/// Wait for one other device on the multicast group.
pub async fn browse_peer(identity: &Identity) -> Result<(DeviceId, SocketAddr), CliError> {
    let presence = LanPresence::bind_multicast(config(identity.device_id(), 0))
        .await
        .map_err(|err| CliError::new(err.to_string()))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        presence
            .browse()
            .await
            .map_err(|err| CliError::new(err.to_string()))?;
        let remain = deadline.saturating_duration_since(Instant::now());
        let incoming = presence
            .recv(remain.min(Duration::from_secs(1)))
            .await
            .map_err(|err| CliError::new(err.to_string()))?;
        if let Some(Incoming::Peer(peer)) = incoming {
            if peer.device_id != identity.device_id() {
                let addr = SocketAddr::new(peer.source.ip(), peer.quic_port);
                eprintln!("lan peer {} {addr}", peer.device_id);
                return Ok((peer.device_id, addr));
            }
        }
    }
    Err(CliError::new("no CBDesk host answered on the LAN"))
}

/// Addresses to dial after a multicast answer.
///
/// The first candidate is loopback when the announce source is some other
/// address. Same-machine CI runners often see a hypervisor address that does
/// not complete a QUIC handshake to a socket bound on `0.0.0.0`. Discovery
/// still uses the multicast packet. The data path may be loopback.
pub fn dial_candidates(source: SocketAddr) -> Vec<SocketAddr> {
    if source.ip().is_loopback() {
        return vec![source];
    }
    let loopback = match source.ip() {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), source.port()),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), source.port()),
    };
    vec![loopback, source]
}

fn config(device_id: DeviceId, quic_port: u16) -> LanPresenceConfig {
    LanPresenceConfig {
        device_id,
        quic_port,
        name: None,
        user_enabled: true,
        profile: NetworkProfile::Private,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_source_is_dialed_after_loopback() {
        let source: SocketAddr = "192.168.64.4:22116".parse().expect("source");
        let addrs = dial_candidates(source);
        assert_eq!(
            addrs[0],
            "127.0.0.1:22116".parse::<SocketAddr>().expect("loop")
        );
        assert_eq!(addrs[1], source);
        let local: SocketAddr = "127.0.0.1:9".parse().expect("local");
        assert_eq!(dial_candidates(local), vec![local]);
        let v6: SocketAddr = "[2001:db8::1]:22116".parse().expect("v6");
        assert_eq!(
            dial_candidates(v6)[0],
            "[::1]:22116".parse::<SocketAddr>().expect("v6 loop")
        );
    }
}

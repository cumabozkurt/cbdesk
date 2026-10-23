// SPDX-License-Identifier: AGPL-3.0-or-later

//! LAN announce and browse for the CLI.
//!
//! `--lan` is the operator opting in. The library still sends nothing when
//! the caller passes a public network profile. This command passes private,
//! because it does not read the OS network category.

use std::net::SocketAddr;
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

fn config(device_id: DeviceId, quic_port: u16) -> LanPresenceConfig {
    LanPresenceConfig {
        device_id,
        quic_port,
        name: None,
        user_enabled: true,
        profile: NetworkProfile::Private,
    }
}

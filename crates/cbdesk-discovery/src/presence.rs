// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime LAN announce and browse.
//!
//! Outbound datagrams use [`lan_should_announce`]. A public network, or a user
//! who turned discovery off, does not send. Receiving is separate: a datagram
//! that arrives is still only a locator. It does not grant a session.
//!
//! Tests pass loopback destinations. The bytes are the same ones a production
//! caller sends to [`MDNS_IPV4`] port [`MDNS_PORT`] and to the broadcast
//! fallback on [`LAN_BROADCAST_PORT`]. [`LanPresence::bind_multicast`] joins
//! that group and binds port 5353 when it is free. The repeating browse
//! schedule is not implemented; a caller decides when to announce again.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use cbdesk_crypto::DeviceId;
use tokio::net::UdpSocket;

use crate::error::DiscoveryError;
use crate::lan::{
    lan_should_announce, LanPacket, NetworkProfile, LAN_BROADCAST_PORT, MDNS_IPV4, MDNS_PORT,
};
use crate::mdns::{decode_mdns, encode_announcement, encode_browse_query, MdnsMessage};

/// Where this process binds, and where outbound datagrams are sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanSocketPlan {
    /// Local mDNS socket. Tests use an ephemeral loopback port.
    pub mdns_bind: SocketAddr,
    /// Local broadcast-fallback socket.
    pub broadcast_bind: SocketAddr,
    /// Peer or group that receives mDNS queries and announcements.
    pub mdns_dest: SocketAddr,
    /// Peer or broadcast address that receives [`LanPacket`] bytes.
    pub broadcast_dest: SocketAddr,
}

impl LanSocketPlan {
    /// Standard IPv4 targets: `224.0.0.251:5353` and `255.255.255.255:22118`.
    ///
    /// The bind addresses are unspecified port 0 so a caller can learn the
    /// actual ports after [`LanPresence::bind`]. Replace them with
    /// `0.0.0.0:5353` and `0.0.0.0:22118` only when those ports are free.
    pub fn ipv4_defaults() -> Self {
        Self {
            mdns_bind: SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, 0)),
            broadcast_bind: SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, 0)),
            mdns_dest: SocketAddr::from((MDNS_IPV4, MDNS_PORT)),
            broadcast_dest: SocketAddr::from((std::net::Ipv4Addr::BROADCAST, LAN_BROADCAST_PORT)),
        }
    }
}

/// What a device is willing to publish on the LAN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanPresenceConfig {
    /// Device that announcements name.
    pub device_id: DeviceId,
    /// QUIC port written into TXT, SRV, and the broadcast body.
    pub quic_port: u16,
    /// Published only when set. Absence means the TXT record has no `name`.
    pub name: Option<String>,
    /// The user asked for LAN discovery.
    pub user_enabled: bool,
    /// Operating-system network class. Public suppresses outbound packets.
    pub profile: NetworkProfile,
}

/// One locator learned from the LAN. This is not an authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredPeer {
    /// Claimed device id. Still authenticate it on the session path.
    pub device_id: DeviceId,
    /// QUIC port from the record, not the UDP source port.
    pub quic_port: u16,
    /// Present only when the peer opted in.
    pub name: Option<String>,
    /// UDP peer that sent the datagram. Use its IP with [`Self::quic_port`].
    pub source: SocketAddr,
    /// Which tested encoding carried the record.
    pub path: DiscoveryPath,
}

/// Which LAN encoding a datagram used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryPath {
    /// DNS-SD response for `_cbdesk._udp.local.`.
    Mdns,
    /// `CBDS` broadcast body.
    Broadcast,
}

/// A datagram the presence socket understood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Incoming {
    /// A peer advertisement.
    Peer(DiscoveredPeer),
    /// A browse query. It does not name a peer.
    BrowseQuery {
        /// UDP source of the query.
        source: SocketAddr,
    },
}

/// Sockets that announce and browse with the tested encodings.
pub struct LanPresence {
    config: LanPresenceConfig,
    mdns: UdpSocket,
    broadcast: UdpSocket,
    mdns_dest: SocketAddr,
    broadcast_dest: SocketAddr,
    joined_multicast: bool,
}

impl LanPresence {
    /// Bind both sockets. Outbound targets may be replaced with [`Self::set_targets`].
    pub async fn bind(
        mut config: LanPresenceConfig,
        plan: LanSocketPlan,
    ) -> Result<Self, DiscoveryError> {
        config.name = normalize_name(config.name)?;
        let mdns = UdpSocket::bind(plan.mdns_bind).await.map_err(lan_io)?;
        let broadcast = UdpSocket::bind(plan.broadcast_bind).await.map_err(lan_io)?;
        broadcast.set_broadcast(true).map_err(lan_io)?;
        Ok(Self {
            config,
            mdns,
            broadcast,
            mdns_dest: plan.mdns_dest,
            broadcast_dest: plan.broadcast_dest,
            joined_multicast: false,
        })
    }

    /// Join `224.0.0.251` and bind port 5353 when that port is free.
    ///
    /// A busy 5353 falls back to an ephemeral port that is still a member of
    /// the group. Sends still aim at `224.0.0.251:5353`, which is the port
    /// other stacks listen on. This socket receives those packets only when
    /// [`Self::mdns_local`] reports port 5353. A public network still sends
    /// nothing.
    pub async fn bind_multicast(mut config: LanPresenceConfig) -> Result<Self, DiscoveryError> {
        config.name = normalize_name(config.name)?;
        let mdns = match bind_v4(
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, MDNS_PORT)),
            Some(MDNS_IPV4),
        ) {
            Ok(socket) => socket,
            Err(_) => bind_v4(
                SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
                Some(MDNS_IPV4),
            )?,
        };
        let broadcast = match bind_v4(
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, LAN_BROADCAST_PORT)),
            None,
        ) {
            Ok(socket) => socket,
            Err(_) => bind_v4(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)), None)?,
        };
        broadcast.set_broadcast(true).map_err(lan_io)?;
        Ok(Self {
            config,
            mdns,
            broadcast,
            mdns_dest: SocketAddr::from((MDNS_IPV4, MDNS_PORT)),
            broadcast_dest: SocketAddr::from((Ipv4Addr::BROADCAST, LAN_BROADCAST_PORT)),
            joined_multicast: true,
        })
    }

    /// Whether [`Self::bind_multicast`] joined `224.0.0.251`.
    pub fn joined_multicast(&self) -> bool {
        self.joined_multicast
    }

    /// Point later sends at a peer. Used by loopback tests and by a caller
    /// that learned a unicast address.
    pub fn set_targets(&mut self, mdns_dest: SocketAddr, broadcast_dest: SocketAddr) {
        self.mdns_dest = mdns_dest;
        self.broadcast_dest = broadcast_dest;
    }

    /// Local mDNS socket, after the kernel chose an ephemeral port.
    pub fn mdns_local(&self) -> Result<SocketAddr, DiscoveryError> {
        self.mdns.local_addr().map_err(lan_io)
    }

    /// Local broadcast-fallback socket.
    pub fn broadcast_local(&self) -> Result<SocketAddr, DiscoveryError> {
        self.broadcast.local_addr().map_err(lan_io)
    }

    /// Whether the next send will leave the machine.
    pub fn announces(&self) -> bool {
        self.allow_outbound()
    }

    /// Send the mDNS announcement and the broadcast body, or send nothing.
    pub async fn announce(&self) -> Result<(), DiscoveryError> {
        self.announce_mdns().await?;
        self.announce_broadcast().await
    }

    /// Send one DNS-SD response, or nothing when the network is public.
    pub async fn announce_mdns(&self) -> Result<(), DiscoveryError> {
        if !self.allow_outbound() {
            return Ok(());
        }
        let bytes = encode_announcement(
            &self.config.device_id,
            self.config.quic_port,
            self.config.name.as_deref(),
        )?;
        self.mdns
            .send_to(&bytes, self.mdns_dest)
            .await
            .map_err(lan_io)?;
        Ok(())
    }

    /// Send one [`LanPacket`], or nothing when the network is public.
    pub async fn announce_broadcast(&self) -> Result<(), DiscoveryError> {
        if !self.allow_outbound() {
            return Ok(());
        }
        let packet = LanPacket {
            device_id: self.config.device_id,
            port: self.config.quic_port,
            name: self.config.name.clone(),
        };
        let bytes = packet.encode()?;
        self.broadcast
            .send_to(&bytes, self.broadcast_dest)
            .await
            .map_err(lan_io)?;
        Ok(())
    }

    /// Send one browse query, or nothing when outbound discovery is suppressed.
    pub async fn browse(&self) -> Result<(), DiscoveryError> {
        if !self.allow_outbound() {
            return Ok(());
        }
        let bytes = encode_browse_query()?;
        self.mdns
            .send_to(&bytes, self.mdns_dest)
            .await
            .map_err(lan_io)?;
        Ok(())
    }

    /// Wait for one understood datagram.
    ///
    /// `Ok(None)` means the timeout elapsed and nothing arrived. A malformed
    /// datagram is an error so a bad encoder is not mistaken for silence.
    pub async fn recv(&self, timeout: Duration) -> Result<Option<Incoming>, DiscoveryError> {
        let mut mdns_buf = [0u8; MAX_DATAGRAM];
        let mut broadcast_buf = [0u8; MAX_DATAGRAM];
        let sleep = tokio::time::sleep(timeout);
        tokio::pin!(sleep);
        tokio::select! {
            () = &mut sleep => Ok(None),
            result = self.mdns.recv_from(&mut mdns_buf) => {
                let (len, source) = result.map_err(lan_io)?;
                Ok(Some(incoming_mdns(&mdns_buf[..len], source)?))
            }
            result = self.broadcast.recv_from(&mut broadcast_buf) => {
                let (len, source) = result.map_err(lan_io)?;
                Ok(Some(incoming_broadcast(&broadcast_buf[..len], source)?))
            }
        }
    }

    fn allow_outbound(&self) -> bool {
        lan_should_announce(self.config.user_enabled, self.config.profile)
    }
}

const MAX_DATAGRAM: usize = 2048;

fn incoming_mdns(bytes: &[u8], source: SocketAddr) -> Result<Incoming, DiscoveryError> {
    match decode_mdns(bytes)? {
        MdnsMessage::Query => Ok(Incoming::BrowseQuery { source }),
        MdnsMessage::Announcement(announcement) => Ok(Incoming::Peer(DiscoveredPeer {
            device_id: announcement.device_id,
            quic_port: announcement.txt.port,
            name: announcement.txt.name,
            source,
            path: DiscoveryPath::Mdns,
        })),
    }
}

fn incoming_broadcast(bytes: &[u8], source: SocketAddr) -> Result<Incoming, DiscoveryError> {
    let packet = LanPacket::decode(bytes)?;
    Ok(Incoming::Peer(DiscoveredPeer {
        device_id: packet.device_id,
        quic_port: packet.port,
        name: packet.name,
        source,
        path: DiscoveryPath::Broadcast,
    }))
}

fn normalize_name(name: Option<String>) -> Result<Option<String>, DiscoveryError> {
    match name {
        None => Ok(None),
        Some(value) if value.is_empty() || value.len() > 63 => Err(DiscoveryError::BadMdns),
        Some(value) => Ok(Some(value)),
    }
}

fn lan_io(err: std::io::Error) -> DiscoveryError {
    DiscoveryError::LanIo(err.to_string())
}

fn bind_v4(addr: SocketAddr, group: Option<Ipv4Addr>) -> Result<UdpSocket, DiscoveryError> {
    let SocketAddr::V4(v4) = addr else {
        return Err(DiscoveryError::LanIo(
            "LAN discovery bind is IPv4".to_string(),
        ));
    };
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )
    .map_err(lan_io)?;
    socket.set_reuse_address(true).map_err(lan_io)?;
    // Linux delivers a multicast copy to every SO_REUSEADDR member. SO_REUSEPORT
    // on Linux can hand the datagram to only one of them, which drops mDNS.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    socket.set_reuse_port(true).map_err(lan_io)?;
    socket.set_nonblocking(true).map_err(lan_io)?;
    socket.bind(&socket2::SockAddr::from(v4)).map_err(lan_io)?;
    let udp = UdpSocket::from_std(socket.into()).map_err(lan_io)?;
    if let Some(group) = group {
        udp.join_multicast_v4(group, Ipv4Addr::UNSPECIFIED)
            .map_err(lan_io)?;
        udp.set_multicast_loop_v4(true).map_err(lan_io)?;
        udp.set_multicast_ttl_v4(255).map_err(lan_io)?;
    }
    Ok(udp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_crypto::Identity;

    fn identity(byte: u8) -> DeviceId {
        Identity::from_seed(&[byte; 32]).expect("seed").device_id()
    }

    async fn pair(
        profile: NetworkProfile,
        user_enabled: bool,
        name: Option<String>,
    ) -> (LanPresence, LanPresence) {
        let plan = LanSocketPlan {
            mdns_bind: "127.0.0.1:0".parse().expect("addr"),
            broadcast_bind: "127.0.0.1:0".parse().expect("addr"),
            mdns_dest: "127.0.0.1:9".parse().expect("addr"),
            broadcast_dest: "127.0.0.1:9".parse().expect("addr"),
        };
        let mut left = LanPresence::bind(
            LanPresenceConfig {
                device_id: identity(3),
                quic_port: 22116,
                name: name.clone(),
                user_enabled,
                profile,
            },
            plan,
        )
        .await
        .expect("bind left");
        let mut right = LanPresence::bind(
            LanPresenceConfig {
                device_id: identity(4),
                quic_port: 22116,
                name,
                user_enabled,
                profile,
            },
            plan,
        )
        .await
        .expect("bind right");
        left.set_targets(
            right.mdns_local().expect("mdns"),
            right.broadcast_local().expect("broadcast"),
        );
        right.set_targets(
            left.mdns_local().expect("mdns"),
            left.broadcast_local().expect("broadcast"),
        );
        (left, right)
    }

    async fn one(profile: NetworkProfile, enabled: bool) -> Incoming {
        let (left, right) = pair(profile, enabled, None).await;
        left.announce_mdns().await.expect("announce");
        right
            .recv(Duration::from_millis(500))
            .await
            .expect("recv")
            .expect("datagram")
    }

    #[tokio::test]
    async fn private_network_exchanges_both_tested_encodings() {
        let (left, right) = pair(NetworkProfile::Private, true, Some("Ofis".to_string())).await;
        left.browse().await.expect("browse");
        match right.recv(Duration::from_millis(500)).await.expect("recv") {
            Some(Incoming::BrowseQuery { source }) => {
                assert!(source.ip().is_loopback());
            }
            other => panic!("expected a browse query, got {other:?}"),
        }

        left.announce_mdns().await.expect("mdns");
        match right.recv(Duration::from_millis(500)).await.expect("recv") {
            Some(Incoming::Peer(peer)) => {
                assert_eq!(peer.path, DiscoveryPath::Mdns);
                assert_eq!(peer.device_id, identity(3));
                assert_eq!(peer.quic_port, 22116);
                assert_eq!(peer.name.as_deref(), Some("Ofis"));
                assert!(peer.source.ip().is_loopback());
            }
            other => panic!("expected an mDNS peer, got {other:?}"),
        }

        left.announce_broadcast().await.expect("broadcast");
        match right.recv(Duration::from_millis(500)).await.expect("recv") {
            Some(Incoming::Peer(peer)) => {
                assert_eq!(peer.path, DiscoveryPath::Broadcast);
                assert_eq!(peer.device_id, identity(3));
                assert_eq!(peer.quic_port, 22116);
                assert_eq!(peer.name.as_deref(), Some("Ofis"));
            }
            other => panic!("expected a broadcast peer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_profile_still_announces_without_a_name() {
        match one(NetworkProfile::Unknown, true).await {
            Incoming::Peer(peer) => {
                assert!(peer.name.is_none());
                assert_eq!(peer.path, DiscoveryPath::Mdns);
            }
            Incoming::BrowseQuery { .. } => panic!("expected a peer"),
        }
    }

    #[tokio::test]
    async fn public_and_disabled_send_nothing() {
        for (profile, enabled) in [
            (NetworkProfile::Public, true),
            (NetworkProfile::Private, false),
        ] {
            let (left, right) = pair(profile, enabled, None).await;
            assert!(!left.announces());
            left.announce().await.expect("announce");
            left.browse().await.expect("browse");
            let found = right.recv(Duration::from_millis(150)).await.expect("recv");
            assert!(found.is_none(), "{profile:?} enabled={enabled}");
        }
    }

    async fn multicast(profile: NetworkProfile, enabled: bool, byte: u8) -> LanPresence {
        LanPresence::bind_multicast(LanPresenceConfig {
            device_id: identity(byte),
            quic_port: 22116,
            name: None,
            user_enabled: enabled,
            profile,
        })
        .await
        .expect("multicast bind")
    }

    async fn expect_peer(presence: &LanPresence, device: u8) {
        match presence.recv(Duration::from_secs(2)).await.expect("recv") {
            Some(Incoming::Peer(peer)) => {
                assert_eq!(peer.path, DiscoveryPath::Mdns);
                assert_eq!(peer.device_id, identity(device));
                assert_eq!(peer.quic_port, 22116);
            }
            other => panic!("expected a multicast announcement, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multicast_group_delivers_the_announcement() {
        // The CLI LAN test also binds port 5353. This listener is only a mutex.
        let _lock = hold_discovery_lock();
        let mut left = multicast(NetworkProfile::Private, true, 3).await;
        assert!(left.joined_multicast());
        let port = left.mdns_local().expect("port").port();
        if port == MDNS_PORT {
            let right = multicast(NetworkProfile::Private, true, 4).await;
            if right.mdns_local().expect("port").port() == MDNS_PORT {
                left.announce_mdns().await.expect("announce");
                expect_peer(&right, 3).await;
            } else {
                loopback_announce(&mut left, port).await;
            }
        } else {
            loopback_announce(&mut left, port).await;
        }
        drop(left);

        let mut quiet = multicast(NetworkProfile::Public, true, 5).await;
        let quiet_port = quiet.mdns_local().expect("port").port();
        quiet.set_targets(
            SocketAddr::from((MDNS_IPV4, quiet_port)),
            "127.0.0.1:9".parse().expect("addr"),
        );
        assert!(!quiet.announces());
        quiet.announce().await.expect("announce");
        quiet.browse().await.expect("browse");
        let found = quiet.recv(Duration::from_millis(200)).await.expect("recv");
        assert!(found.is_none());
    }

    async fn loopback_announce(presence: &mut LanPresence, port: u16) {
        presence.set_targets(
            SocketAddr::from((MDNS_IPV4, port)),
            "127.0.0.1:9".parse().expect("addr"),
        );
        presence.announce_mdns().await.expect("announce");
        expect_peer(presence, 3).await;
    }

    fn hold_discovery_lock() -> std::net::TcpListener {
        let started = std::time::Instant::now();
        loop {
            if let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:22953") {
                return listener;
            }
            if started.elapsed() > Duration::from_secs(30) {
                panic!("timed out waiting for the discovery test lock");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Local network diagnosis and a mapping classifier.
//!
//! [`local_report`] looks at this machine only. It binds UDP sockets and asks
//! the kernel which source address it would use toward a documentation
//! prefix. Those calls do not transmit a datagram. The report does not claim
//! a NAT type when the external mapping was not observed.
//!
//! [`classify_mappings`] is the decision rule for endpoint-independent versus
//! endpoint-dependent (symmetric) mappings. A live symmetric NAT is not
//! sampled here. [`measure_loopback_paths`] connects two endpoints on this
//! machine and records whether iroh selected a direct path. That is not a
//! hole punch across a NAT.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use cbdesk_crypto::Identity;
use zeroize::Zeroize;

use crate::error::NetError;
use crate::policy::EndpointConfig;
use crate::quic::QuicEndpoint;

/// What this process could see about its own addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NatClass {
    /// Neither IPv4 nor IPv6 UDP could be bound.
    UdpBlocked,
    /// The IPv4 source chosen for a documentation prefix is in `100.64.0.0/10`.
    ///
    /// The external mapping is still unknown.
    Cgnat,
    /// A source address is a public unicast address on this machine.
    ///
    /// This is not a measurement of hole punching.
    OpenInternet,
    /// The source is loopback, private, or missing. The mapping is unknown.
    PrivateUnmeasured,
}

/// Whether a symmetric-NAT sample exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetricNatStatus {
    /// No external mapping was observed.
    Unmeasured,
}

/// One observed external mapping.
///
/// `local_port` is the socket's local port. `external_port` is the port a
/// peer saw. Two remotes and two external ports for one local port are an
/// endpoint-dependent mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappingObservation {
    /// Local UDP port.
    pub local_port: u16,
    /// Remote socket the mapping was learned for.
    pub remote: SocketAddr,
    /// Port the remote peer observed.
    pub external_port: u16,
}

/// How a set of observations maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingKind {
    /// One local port kept the same external port toward every remote.
    EndpointIndependent,
    /// One local port used different external ports toward different remotes.
    ///
    /// Direct hole punching often fails in this case, so a relay is required.
    EndpointDependent,
}

/// Diagnosis of this machine. Addresses are local source addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetCheckReport {
    /// `0.0.0.0:0` could be bound.
    pub udp_ipv4: bool,
    /// `[::]:0` could be bound.
    pub udp_ipv6: bool,
    /// Source address toward `192.0.2.1`, when the kernel selected one.
    pub ipv4_source: Option<IpAddr>,
    /// Source address toward `2001:db8::1`, when the kernel selected one.
    pub ipv6_source: Option<IpAddr>,
    /// [`NatClass::Cgnat`] when the IPv4 source is in the shared space.
    pub cgnat: bool,
    /// Address class. Not a hole-punch result.
    pub nat_class: NatClass,
    /// Always [`SymmetricNatStatus::Unmeasured`] from this function.
    pub symmetric_nat: SymmetricNatStatus,
    /// How many relay URLs the caller said are configured. They are not dialed.
    pub configured_relays: usize,
    /// UPnP was not queried.
    pub upnp: &'static str,
    /// No DHT node was contacted.
    pub dht: &'static str,
    /// Hole punching was not attempted.
    pub hole_punch: &'static str,
}

/// Result of connecting two endpoints on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopbackPathReport {
    /// Direct IP paths iroh reported.
    pub direct_paths: usize,
    /// Relay paths iroh reported.
    pub relay_paths: usize,
    /// The selected path was a direct IP path.
    pub selected_direct: bool,
    /// Fixed explanation. This is not a NAT measurement.
    pub note: &'static str,
}

const HOLE_PUNCH_NOTE: &str = "not measured; this command does not send a hole-punch attempt";
const LOOPBACK_NOTE: &str =
    "both sockets are on this machine; a direct path here is not a NAT hole punch";

/// Build a report without transmitting a datagram.
pub fn local_report(configured_relays: usize) -> NetCheckReport {
    let udp_ipv4 = UdpSocket::bind("0.0.0.0:0").is_ok();
    let udp_ipv6 = UdpSocket::bind("[::]:0").is_ok();
    let ipv4_source = preferred_source(SocketAddr::from((Ipv4Addr::new(192, 0, 2, 1), 9)));
    let ipv6_source = preferred_source(SocketAddr::from((
        Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        9,
    )));
    let cgnat = ipv4_source.is_some_and(is_cgnat);
    let nat_class = classify_local(udp_ipv4, udp_ipv6, ipv4_source, ipv6_source);
    NetCheckReport {
        udp_ipv4,
        udp_ipv6,
        ipv4_source,
        ipv6_source,
        cgnat,
        nat_class,
        symmetric_nat: SymmetricNatStatus::Unmeasured,
        configured_relays,
        upnp: "not probed",
        dht: "not probed",
        hole_punch: HOLE_PUNCH_NOTE,
    }
}

/// Classify observations of one local port.
///
/// Fewer than two remotes, or more than one local port, is an error. The
/// caller has not supplied a sample this function can judge.
pub fn classify_mappings(observations: &[MappingObservation]) -> Result<MappingKind, NetError> {
    if observations.len() < 2 {
        return Err(NetError::Quic(
            "a mapping class needs two observations".to_string(),
        ));
    }
    let local = observations[0].local_port;
    if observations.iter().any(|item| item.local_port != local) {
        return Err(NetError::Quic(
            "mapping observations must share one local port".to_string(),
        ));
    }
    let mut remotes = Vec::new();
    let mut ports = Vec::new();
    for item in observations {
        if !remotes.contains(&item.remote) {
            remotes.push(item.remote);
            ports.push(item.external_port);
        }
    }
    if remotes.len() < 2 {
        return Err(NetError::Quic(
            "a mapping class needs two remote endpoints".to_string(),
        ));
    }
    if ports.windows(2).all(|pair| pair[0] == pair[1]) {
        Ok(MappingKind::EndpointIndependent)
    } else {
        Ok(MappingKind::EndpointDependent)
    }
}

/// Dial two loopback endpoints and read iroh's path list.
///
/// The endpoints use an empty relay list. The selected path on a successful
/// dial is direct because both sockets are local. This function does not
/// measure symmetric NAT.
pub async fn measure_loopback_paths() -> Result<LoopbackPathReport, NetError> {
    let host_identity =
        Identity::generate().map_err(|_| NetError::Quic("random source failed".to_string()))?;
    let client_identity =
        Identity::generate().map_err(|_| NetError::Quic("random source failed".to_string()))?;
    let mut host_seed = host_identity.signing_seed();
    let mut client_seed = client_identity.signing_seed();
    let addr: SocketAddr = "127.0.0.1:0"
        .parse()
        .map_err(|_| NetError::Quic("loopback address is invalid".to_string()))?;
    let config = EndpointConfig::default();
    let host = QuicEndpoint::bind(&config, addr, &host_seed).await?;
    let client = QuicEndpoint::bind(&config, addr, &client_seed).await?;
    host_seed.zeroize();
    client_seed.zeroize();
    let host_addr = host.local_addr()?;
    let host_key = host.public_key();
    let host_task = tokio::spawn(async move {
        let conn = host.accept().await?;
        let observed = conn.observe_paths();
        conn.probe_reflect(64).await?;
        Ok::<_, NetError>(observed)
    });
    let conn = client.connect(&host_key, host_addr).await?;
    let client_paths = conn.observe_paths();
    conn.probe_send(b"path", 64).await?;
    let host_paths = host_task
        .await
        .map_err(|_| NetError::Quic("loopback accept task failed".to_string()))??;
    let _ = host_paths;
    Ok(LoopbackPathReport {
        direct_paths: client_paths.direct_paths,
        relay_paths: client_paths.relay_paths,
        selected_direct: client_paths.selected_direct,
        note: LOOPBACK_NOTE,
    })
}

fn preferred_source(remote: SocketAddr) -> Option<IpAddr> {
    let bind = if remote.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind).ok()?;
    socket.connect(remote).ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}

fn classify_local(
    udp_ipv4: bool,
    udp_ipv6: bool,
    ipv4_source: Option<IpAddr>,
    ipv6_source: Option<IpAddr>,
) -> NatClass {
    if !udp_ipv4 && !udp_ipv6 {
        return NatClass::UdpBlocked;
    }
    if ipv4_source.is_some_and(is_cgnat) {
        return NatClass::Cgnat;
    }
    if ipv4_source.is_some_and(is_public_unicast) || ipv6_source.is_some_and(is_public_unicast) {
        return NatClass::OpenInternet;
    }
    NatClass::PrivateUnmeasured
}

fn is_cgnat(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            a == 100 && (64..=127).contains(&b)
        }
        IpAddr::V6(_) => false,
    }
}

fn is_public_unicast(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !is_cgnat(IpAddr::V4(ip))
                && !is_documentation_v4(ip)
        }
        IpAddr::V6(ip) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && !is_unique_local(ip)
                && !is_documentation_v6(ip)
                && (ip.segments()[0] & 0xffc0) != 0xfe80
        }
    }
}

fn is_documentation_v4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    (a == 192 && b == 0 && ip.octets()[2] == 2)
        || (a == 198 && b == 51 && ip.octets()[2] == 100)
        || (a == 203 && b == 0 && ip.octets()[2] == 113)
}

fn is_documentation_v6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

fn is_unique_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xfe00 == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_address_space_is_cgnat_and_a_lan_address_is_not_public() {
        assert!(is_cgnat(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_cgnat(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
        assert!(!is_cgnat(IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1))));
        assert!(!is_cgnat(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1))));
        assert!(!is_public_unicast(IpAddr::V4(Ipv4Addr::new(
            192, 168, 1, 9
        ))));
        assert!(!is_public_unicast(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1))));
        assert!(!is_public_unicast(IpAddr::V4(Ipv4Addr::new(100, 64, 1, 1))));
        assert!(!is_public_unicast(IpAddr::V4(Ipv4Addr::new(
            203, 0, 113, 10
        ))));
        assert!(!is_public_unicast(IpAddr::V4(Ipv4Addr::new(
            198, 51, 100, 1
        ))));
        assert!(is_public_unicast(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn endpoint_dependent_mappings_are_the_symmetric_case() {
        let local = 40000;
        let left = "203.0.113.1:1".parse().expect("left");
        let right = "203.0.113.2:1".parse().expect("right");
        let independent = [
            MappingObservation {
                local_port: local,
                remote: left,
                external_port: 50000,
            },
            MappingObservation {
                local_port: local,
                remote: right,
                external_port: 50000,
            },
        ];
        assert_eq!(
            classify_mappings(&independent).expect("class"),
            MappingKind::EndpointIndependent
        );
        let dependent = [
            independent[0],
            MappingObservation {
                external_port: 50001,
                ..independent[1]
            },
        ];
        assert_eq!(
            classify_mappings(&dependent).expect("class"),
            MappingKind::EndpointDependent
        );
        assert!(classify_mappings(&independent[..1]).is_err());
    }

    #[test]
    fn the_local_report_does_not_claim_a_symmetric_nat() {
        let report = local_report(0);
        assert_eq!(report.symmetric_nat, SymmetricNatStatus::Unmeasured);
        assert_eq!(report.upnp, "not probed");
        assert_eq!(report.dht, "not probed");
        assert!(report.hole_punch.contains("not measured"));
        assert!(report.udp_ipv4 || report.nat_class == NatClass::UdpBlocked);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn loopback_paths_are_direct_and_labeled_as_not_a_nat_measurement() {
        let report = measure_loopback_paths().await.expect("paths");
        assert!(report.direct_paths >= 1, "{report:?}");
        assert_eq!(report.relay_paths, 0);
        assert!(report.selected_direct);
        assert!(report.note.contains("not a NAT hole punch"));
    }
}

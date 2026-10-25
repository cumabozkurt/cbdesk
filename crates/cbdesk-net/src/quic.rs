// SPDX-License-Identifier: AGPL-3.0-or-later

//! QUIC using iroh without n0 defaults.
//!
//! iroh 1.4.1 has no `Endpoint::empty_builder`. The equivalent is
//! [`iroh::endpoint::presets::Minimal`], which sets only the TLS provider,
//! plus an empty address-lookup list, a disabled port mapper, and a DNS
//! resolver that never leaves the process. The default relay mode is
//! [`iroh::RelayMode::Disabled`]. A non-empty operator list installs
//! [`iroh::RelayMode::Custom`] after the forbidden-suffix check.
//! `presets::N0`, `RelayMode::Default`, and `RelayMode::Staging` are not
//! used. See ADR-006.
//!
//! T-NET-001 for this build is a loopback echo. The resolver records every
//! name it is asked for and answers NXDOMAIN, so a query for a forbidden
//! suffix fails the test. This is not a capture of every packet on the
//! machine. A user-run relay is a separate test and is not an n0 server.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cbdesk_proto::ALPN;
use iroh::dns::{BoxIter, DnsError, DnsResolver, Resolver, TxtRecordData};
use iroh::endpoint::{presets, PortmapperConfig};
use iroh::tls::CaTlsConfig;
use iroh::{Endpoint, EndpointAddr, EndpointId, NetReportConfig, RelayMode, RelayUrl, SecretKey};

use crate::error::NetError;
use crate::policy::{validate_endpoint, EndpointConfig};

/// A QUIC endpoint that dials by IP and does not install n0 relays.
#[derive(Clone)]
pub struct QuicEndpoint {
    inner: Endpoint,
    dns_queries: Arc<Mutex<Vec<String>>>,
    relays: Vec<String>,
}

/// One accepted or dialed QUIC connection.
///
/// Accepting this connection does not grant control of the computer. The
/// session layer still has to run consent.
pub struct QuicConn {
    inner: iroh::endpoint::Connection,
}

impl std::fmt::Debug for QuicEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QuicEndpoint")
            .field("public_key", &self.public_key())
            .finish()
    }
}

impl std::fmt::Debug for QuicConn {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("QuicConn")
    }
}

impl QuicEndpoint {
    /// Bind a direct endpoint.
    ///
    /// `seed` is the 32-byte Ed25519 seed. iroh's endpoint id is that key's
    /// public half. `addr` is the only socket the endpoint binds; pass
    /// `127.0.0.1:0` in tests and `0.0.0.0:22116` when a fixed QUIC port is
    /// required. An empty relay list stays direct. A non-empty list must
    /// already have passed [`validate_endpoint`]. `RelayMode::Default` and
    /// `RelayMode::Staging` are not selected. Loopback relays skip certificate
    /// verification because a user-run relay on this machine presents a
    /// certificate this process did not obtain from a public authority.
    pub async fn bind(
        config: &EndpointConfig,
        addr: SocketAddr,
        seed: &[u8; 32],
    ) -> Result<Self, NetError> {
        validate_endpoint(config)?;
        if config.relay_only && config.relays.is_empty() {
            return Err(NetError::RelayOnlyNeedsRelay);
        }
        let mode = relay_mode(config)?;
        let (resolver, dns_queries) = recording_resolver();
        let mut builder = Endpoint::builder(presets::Minimal)
            .secret_key(SecretKey::from_bytes(seed))
            .clear_address_lookup()
            .relay_mode(mode)
            .portmapper_config(PortmapperConfig::Disabled)
            .net_report_config(net_report_config(config))
            .dns_resolver(resolver)
            .alpns(vec![ALPN.as_bytes().to_vec()])
            .clear_ip_transports();
        if relays_are_loopback(&config.relays) {
            builder = builder.ca_tls_config(loopback_tls_config());
        }
        if !config.relay_only {
            builder = builder
                .bind_addr(addr.to_string())
                .map_err(|err| NetError::Quic(err.to_string()))?;
        }
        let inner = builder
            .bind()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        Ok(Self {
            inner,
            dns_queries,
            relays: config.relays.clone(),
        })
    }

    /// Ed25519 public key bytes for this endpoint.
    pub fn public_key(&self) -> [u8; 32] {
        *self.inner.id().as_bytes()
    }

    /// Bound IPv4 socket, including the port chosen for port 0.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.inner
            .bound_sockets()
            .into_iter()
            .find(|addr| addr.is_ipv4())
            .ok_or_else(|| NetError::Quic("endpoint has no IPv4 socket".to_string()))
    }

    /// Relay URLs this endpoint was built with. Empty unless the operator set them.
    pub fn relay_urls(&self) -> Vec<String> {
        self.relays.clone()
    }

    /// Whether `url` is one of the relays passed to [`Self::bind`].
    pub fn allows_relay(&self, url: &str) -> bool {
        let Ok(wanted) = url.parse::<RelayUrl>() else {
            return false;
        };
        self.relays
            .iter()
            .any(|item| item.parse::<RelayUrl>().ok().as_ref() == Some(&wanted))
    }

    /// Wait until a configured relay is connected.
    ///
    /// iroh chooses that relay from the HTTPS probe in the net report. This
    /// waits indefinitely when every configured relay is unreachable. An
    /// endpoint with an empty relay list has nothing to connect to.
    pub async fn online(&self) {
        self.inner.online().await;
    }

    /// DNS names the endpoint's resolver was asked to look up.
    pub fn dns_queries(&self) -> Vec<String> {
        match self.dns_queries.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Dial `peer` at `addr` over the direct IP path.
    pub async fn connect(&self, peer: &[u8; 32], addr: SocketAddr) -> Result<QuicConn, NetError> {
        self.connect_path(peer, Some(addr), None).await
    }

    /// Dial `peer` at an optional direct address and an optional user relay.
    ///
    /// `relay` must be one of the URLs passed to [`Self::bind`]. A forbidden
    /// suffix still fails. Passing neither address nor relay is
    /// [`NetError::NoCandidate`].
    pub async fn connect_path(
        &self,
        peer: &[u8; 32],
        addr: Option<SocketAddr>,
        relay: Option<&str>,
    ) -> Result<QuicConn, NetError> {
        if peer == &self.public_key() {
            return Err(NetError::SelfDial);
        }
        if addr.is_none() && relay.is_none() {
            return Err(NetError::NoCandidate);
        }
        let id = EndpointId::from_bytes(peer)
            .map_err(|_| NetError::Quic("peer key is not a valid Ed25519 point".to_string()))?;
        let mut endpoint_addr = EndpointAddr::new(id);
        if let Some(addr) = addr {
            endpoint_addr = endpoint_addr.with_ip_addr(addr);
        }
        if let Some(relay) = relay {
            if !self.allows_relay(relay) {
                return Err(NetError::RelaysNotInstalled);
            }
            crate::policy::check_hints(&[crate::policy::RelayHint {
                url: relay.to_string(),
            }])?;
            let url: RelayUrl = relay
                .parse()
                .map_err(|_| NetError::BadRelay(relay.to_string()))?;
            endpoint_addr = endpoint_addr.with_relay_url(url);
        }
        let inner = self
            .inner
            .connect(endpoint_addr, ALPN.as_bytes())
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        Ok(QuicConn { inner })
    }

    /// Accept the next QUIC connection for `cbdesk/1`.
    ///
    /// The returned future ends with [`NetError::Closed`] after [`Self::close`].
    pub async fn accept(&self) -> Result<QuicConn, NetError> {
        let incoming = self.inner.accept().await.ok_or(NetError::Closed)?;
        let inner = incoming
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        Ok(QuicConn { inner })
    }

    /// Stop accepting and close active connections.
    pub async fn close(&self) {
        self.inner.close().await;
    }
}

impl QuicConn {
    /// Send `payload` on a new bidirectional stream and read the reply.
    ///
    /// This is a connectivity probe, not a session message. The probe refuses
    /// payloads above 64 KiB.
    pub async fn probe_send(&self, payload: &[u8], limit: usize) -> Result<Vec<u8>, NetError> {
        check_probe(payload.len(), limit)?;
        let (mut send, mut recv) = self
            .inner
            .open_bi()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        send.write_all(payload)
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        send.finish()
            .map_err(|err| NetError::Quic(err.to_string()))?;
        let reply = recv
            .read_to_end(limit)
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        // The reflector is waiting for this close. Closing first would let the
        // peer discard bytes that have not reached `read_to_end` yet.
        self.inner.close(0u32.into(), b"probe");
        Ok(reply)
    }

    /// Read one bidirectional stream, write those bytes back, and wait.
    ///
    /// The wait ends when the peer closes the connection. [`Self::probe_send`]
    /// closes after it has read the reply. Returning earlier can drop that
    /// reply before the peer's QUIC stack delivers it.
    pub async fn probe_reflect(&self, limit: usize) -> Result<(), NetError> {
        check_probe(0, limit)?;
        let (mut send, mut recv) = self
            .inner
            .accept_bi()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        let data = recv
            .read_to_end(limit)
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        send.write_all(&data)
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        send.finish()
            .map_err(|err| NetError::Quic(err.to_string()))?;
        // Stay up until the peer has read the reply and closed. Dropping the
        // connection here can discard that reply.
        let _reason = self.inner.closed().await;
        Ok(())
    }

    pub(crate) async fn open_bi_streams(
        &self,
    ) -> Result<(iroh::endpoint::SendStream, iroh::endpoint::RecvStream), NetError> {
        self.inner
            .open_bi()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))
    }

    pub(crate) async fn accept_bi_streams(
        &self,
    ) -> Result<(iroh::endpoint::SendStream, iroh::endpoint::RecvStream), NetError> {
        self.inner
            .accept_bi()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))
    }

    pub(crate) async fn open_uni_stream(&self) -> Result<iroh::endpoint::SendStream, NetError> {
        self.inner
            .open_uni()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))
    }

    pub(crate) async fn accept_uni_stream(&self) -> Result<iroh::endpoint::RecvStream, NetError> {
        self.inner
            .accept_uni()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))
    }

    /// Ed25519 public key of the peer that completed the QUIC handshake.
    pub(crate) fn remote_key(&self) -> [u8; 32] {
        *self.inner.remote_id().as_bytes()
    }

    /// The peer's Ed25519 public key. This is the device id, not a grant.
    pub fn remote_device_id(&self) -> [u8; 32] {
        self.remote_key()
    }

    /// Snapshot of the open paths.
    ///
    /// A direct path on loopback is not a NAT hole punch. Relay paths appear
    /// only when this connection was dialed through a relay.
    pub fn observe_paths(&self) -> PathObservation {
        let paths = self.inner.paths();
        let mut observed = PathObservation {
            direct_paths: 0,
            relay_paths: 0,
            selected_direct: false,
            selected_relay: false,
        };
        for path in paths.iter() {
            if path.is_ip() {
                observed.direct_paths = observed.direct_paths.saturating_add(1);
                observed.selected_direct |= path.is_selected();
            }
            if path.is_relay() {
                observed.relay_paths = observed.relay_paths.saturating_add(1);
                observed.selected_relay |= path.is_selected();
            }
        }
        observed
    }
}

/// Open paths on one QUIC connection, counted at a single instant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PathObservation {
    /// Paths whose remote address is an IP socket.
    pub direct_paths: usize,
    /// Paths whose remote address is a relay.
    pub relay_paths: usize,
    /// The path selected for application data is a direct IP path.
    pub selected_direct: bool,
    /// The path selected for application data is a relay path.
    pub selected_relay: bool,
}

const PROBE_MAX: usize = 64 * 1024;

fn check_probe(payload_len: usize, limit: usize) -> Result<(), NetError> {
    if payload_len > PROBE_MAX || limit > PROBE_MAX || limit < payload_len {
        return Err(NetError::Quic("probe is limited to 64 KiB".to_string()));
    }
    Ok(())
}

fn net_report_config(config: &EndpointConfig) -> NetReportConfig {
    if config.relays.is_empty() {
        // Nothing to measure. Leave the probes off so a direct endpoint does
        // not open report traffic.
        return NetReportConfig::minimal();
    }
    // A home relay is chosen from this probe. The probe list is the operator's
    // relays. The captive-portal check dials port 80 on the relay host, which
    // is not the relay port, so it stays off.
    let mut report = NetReportConfig::default();
    report.captive_portal_check = false;
    report
}

fn relay_mode(config: &EndpointConfig) -> Result<RelayMode, NetError> {
    if config.relays.is_empty() {
        return Ok(RelayMode::Disabled);
    }
    let mut urls = Vec::with_capacity(config.relays.len());
    for text in &config.relays {
        let url = text
            .parse::<RelayUrl>()
            .map_err(|_| NetError::BadRelay(text.clone()))?;
        urls.push(url);
    }
    Ok(RelayMode::custom(urls))
}

fn loopback_tls_config() -> CaTlsConfig {
    CaTlsConfig::custom_server_cert_verifier(Arc::new(|provider| {
        Ok(Arc::new(LoopbackRelayCerts { provider }))
    }))
}

/// Accepts any certificate. Installed only when every configured relay host
/// is a literal loopback address, which is how a user-run relay on this
/// machine presents a certificate this process did not obtain from a public
/// authority.
#[derive(Debug)]
struct LoopbackRelayCerts {
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for LoopbackRelayCerts {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn relays_are_loopback(relays: &[String]) -> bool {
    !relays.is_empty()
        && relays.iter().all(|text| {
            crate::policy::relay_host(text).is_some_and(|host| {
                host.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            })
        })
}

fn recording_resolver() -> (DnsResolver, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let resolver = DnsResolver::custom(RecordingResolver {
        seen: Arc::clone(&seen),
    });
    (resolver, seen)
}

#[derive(Clone, Debug)]
struct RecordingResolver {
    seen: Arc<Mutex<Vec<String>>>,
}

type Lookup<T> = Pin<Box<dyn std::future::Future<Output = Result<BoxIter<T>, DnsError>> + Send>>;

impl RecordingResolver {
    fn note(&self, host: &str) {
        match self.seen.lock() {
            Ok(mut guard) => guard.push(host.to_string()),
            Err(poisoned) => poisoned.into_inner().push(host.to_string()),
        }
    }

    fn refuse<T>(&self, host: String) -> Lookup<T> {
        self.note(&host);
        Box::pin(async { Err(n0_error::e!(DnsError::NxDomain)) })
    }
}

impl Resolver for RecordingResolver {
    fn lookup_ipv4(&self, host: String) -> Lookup<Ipv4Addr> {
        self.refuse(host)
    }

    fn lookup_ipv6(&self, host: String) -> Lookup<Ipv6Addr> {
        self.refuse(host)
    }

    fn lookup_txt(&self, host: String) -> Lookup<TxtRecordData> {
        self.refuse(host)
    }

    fn clear_cache(&self) {}

    fn reset(&self) -> Box<dyn Resolver> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use cbdesk_crypto::Identity;

    use super::*;
    use crate::policy::host_forbidden;

    fn seed(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    async fn bind_loopback(byte: u8) -> QuicEndpoint {
        let addr = "127.0.0.1:0".parse().expect("addr");
        QuicEndpoint::bind(&EndpointConfig::default(), addr, &seed(byte))
            .await
            .expect("bind")
    }

    #[tokio::test]
    async fn rejected_configs_do_not_bind() {
        let addr = "127.0.0.1:0".parse().expect("addr");
        let defaults = EndpointConfig {
            relays: Vec::new(),
            use_n0_defaults: true,
            relay_only: false,
            isolated: false,
        };
        assert_eq!(
            QuicEndpoint::bind(&defaults, addr, &seed(1))
                .await
                .expect_err("n0"),
            NetError::N0DefaultsForbidden
        );
        let forbidden = EndpointConfig {
            relays: vec!["https://foo.iroh.link:443".to_string()],
            use_n0_defaults: false,
            relay_only: false,
            isolated: false,
        };
        assert!(matches!(
            QuicEndpoint::bind(&forbidden, addr, &seed(1)).await,
            Err(NetError::ForbiddenRelay { .. })
        ));
        let operator = EndpointConfig {
            relays: vec!["https://203.0.113.9:443".to_string()],
            use_n0_defaults: false,
            relay_only: false,
            isolated: false,
        };
        let bound = QuicEndpoint::bind(&operator, addr, &seed(1))
            .await
            .expect("operator relay");
        assert_eq!(
            bound.relay_urls(),
            vec!["https://203.0.113.9:443".to_string()]
        );
        assert!(bound.dns_queries().is_empty(), "{:?}", bound.dns_queries());
        bound.close().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn direct_echo_asks_dns_for_nothing() {
        let server_identity = Identity::from_seed(&seed(7)).expect("server");
        let client_identity = Identity::from_seed(&seed(8)).expect("client");
        let server = bind_loopback(7).await;
        let client = bind_loopback(8).await;
        assert_eq!(server.public_key(), *server_identity.device_id().as_bytes());
        assert_eq!(client.public_key(), *client_identity.device_id().as_bytes());
        assert!(server.relay_urls().is_empty());
        assert!(client.relay_urls().is_empty());
        let server_addr = server.local_addr().expect("addr");
        assert!(server_addr.ip().is_loopback());
        assert_ne!(server_addr.port(), 0);

        let self_dial = client
            .connect(&client.public_key(), client.local_addr().expect("addr"))
            .await;
        assert_eq!(self_dial.expect_err("self"), NetError::SelfDial);

        let server_key = server.public_key();
        let server_for_accept = server.clone();
        let client_for_dial = client.clone();
        let reflect = async move {
            let conn = server_for_accept.accept().await?;
            conn.probe_reflect(64).await
        };
        let dial = async move {
            let conn = client_for_dial.connect(&server_key, server_addr).await?;
            conn.probe_send(b"cbdesk", 64).await
        };
        let (reflected, echoed) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(15), reflect),
            tokio::time::timeout(Duration::from_secs(15), dial),
        );
        let echoed = echoed.expect("echo timed out").expect("echo");
        reflected.expect("reflect timed out").expect("reflect");
        assert_eq!(echoed, b"cbdesk");

        for endpoint in [&server, &client] {
            let queries = endpoint.dns_queries();
            assert!(queries.is_empty(), "direct echo queried DNS: {queries:?}");
            for name in &queries {
                assert!(!host_forbidden(name), "forbidden DNS name {name}");
            }
        }
        server.close().await;
        client.close().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn user_relay_carries_a_probe_without_a_udp_socket() {
        let (_map, url, server) = iroh::test_utils::run_relay_server().await.expect("relay");
        let text = url.to_string();
        let config = EndpointConfig {
            relays: vec![text.clone()],
            use_n0_defaults: false,
            relay_only: true,
            isolated: false,
        };
        let host_seed = seed(71);
        let client_seed = seed(72);
        let addr = "127.0.0.1:0".parse().expect("addr");
        let host = QuicEndpoint::bind(&config, addr, &host_seed)
            .await
            .expect("host relay");
        let client = QuicEndpoint::bind(&config, addr, &client_seed)
            .await
            .expect("client relay");
        let identity = Identity::from_seed(&host_seed).expect("identity");
        assert_eq!(host.public_key(), *identity.device_id().as_bytes());
        let ready = tokio::time::timeout(Duration::from_secs(8), async {
            tokio::join!(host.online(), client.online());
        })
        .await;
        assert!(ready.is_ok(), "user relay did not connect");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let ticket = cbdesk_discovery::sign_ticket(
            cbdesk_proto::InviteTicket {
                relay_urls: vec![text],
                expires_at: now.saturating_add(600),
                purpose: cbdesk_proto::InvitePurpose::Session,
                ..cbdesk_proto::InviteTicket::default()
            },
            &identity,
        )
        .expect("ticket");
        let mut replay = cbdesk_discovery::ReplayCache::new();
        let reflect = async {
            let conn = host.accept().await?;
            conn.probe_reflect(64).await
        };
        let dial = async {
            let conn = crate::dial::dial_verified_ticket(&client, &ticket, now, &mut replay)
                .await
                .map_err(|err| NetError::Quic(err.to_string()))?;
            conn.probe_send(b"relay", 64).await
        };
        let (reflected, echoed) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(8), reflect),
            tokio::time::timeout(Duration::from_secs(8), dial),
        );
        let echoed = echoed.expect("relay echo timed out").expect("echo");
        reflected
            .expect("relay reflect timed out")
            .expect("reflect");
        assert_eq!(echoed, b"relay");
        assert!(host.dns_queries().is_empty(), "{:?}", host.dns_queries());
        assert!(
            client.dns_queries().is_empty(),
            "{:?}",
            client.dns_queries()
        );
        host.close().await;
        client.close().await;
        drop(server);
    }
}

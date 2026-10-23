// SPDX-License-Identifier: AGPL-3.0-or-later

//! Direct QUIC using iroh without n0 defaults.
//!
//! iroh 1.4.1 has no `Endpoint::empty_builder`. The equivalent is
//! [`iroh::endpoint::presets::Minimal`], which sets only the TLS provider,
//! plus [`iroh::RelayMode::Disabled`], an empty address-lookup list, a
//! disabled port mapper, and a DNS resolver that never leaves the process.
//! `presets::N0` is not used: that preset publishes to and resolves from
//! `iroh.link`. See ADR-006.
//!
//! T-NET-001 for this build is a loopback echo. The resolver records every
//! name it is asked for and answers NXDOMAIN, so a query for a forbidden
//! suffix fails the test. This is not a capture of every packet on the
//! machine.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cbdesk_proto::ALPN;
use iroh::dns::{BoxIter, DnsError, DnsResolver, Resolver, TxtRecordData};
use iroh::endpoint::{presets, PortmapperConfig};
use iroh::{Endpoint, EndpointAddr, EndpointId, NetReportConfig, RelayMode, SecretKey};

use crate::error::NetError;
use crate::policy::{validate_endpoint, EndpointConfig};

/// A QUIC endpoint that dials by IP and does not install n0 relays.
#[derive(Clone)]
pub struct QuicEndpoint {
    inner: Endpoint,
    dns_queries: Arc<Mutex<Vec<String>>>,
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
    /// required. Operator relays are rejected: this builder cannot install
    /// them without also growing a relay allow-list, which this slice does
    /// not have.
    pub async fn bind(
        config: &EndpointConfig,
        addr: SocketAddr,
        seed: &[u8; 32],
    ) -> Result<Self, NetError> {
        validate_endpoint(config)?;
        if !config.relays.is_empty() {
            return Err(NetError::RelaysNotInstalled);
        }
        let (resolver, dns_queries) = recording_resolver();
        let builder = Endpoint::builder(presets::Minimal)
            .secret_key(SecretKey::from_bytes(seed))
            .clear_address_lookup()
            .relay_mode(RelayMode::Disabled)
            .portmapper_config(PortmapperConfig::Disabled)
            .net_report_config(NetReportConfig::minimal())
            .dns_resolver(resolver)
            .alpns(vec![ALPN.as_bytes().to_vec()])
            .clear_ip_transports()
            .bind_addr(addr.to_string())
            .map_err(|err| NetError::Quic(err.to_string()))?;
        let inner = builder
            .bind()
            .await
            .map_err(|err| NetError::Quic(err.to_string()))?;
        Ok(Self { inner, dns_queries })
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

    /// Relay URLs currently advertised. Empty for this builder.
    pub fn relay_urls(&self) -> Vec<String> {
        self.inner
            .addr()
            .relay_urls()
            .map(ToString::to_string)
            .collect()
    }

    /// DNS names the endpoint's resolver was asked to look up.
    pub fn dns_queries(&self) -> Vec<String> {
        match self.dns_queries.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Dial `peer` at `addr` over the direct IP path.
    ///
    /// No relay URL is added. A forbidden suffix cannot appear because the
    /// address is an IP socket, not a hostname.
    pub async fn connect(&self, peer: &[u8; 32], addr: SocketAddr) -> Result<QuicConn, NetError> {
        if peer == &self.public_key() {
            return Err(NetError::SelfDial);
        }
        let id = EndpointId::from_bytes(peer)
            .map_err(|_| NetError::Quic("peer key is not a valid Ed25519 point".to_string()))?;
        let endpoint_addr = EndpointAddr::new(id).with_ip_addr(addr);
        if endpoint_addr.relay_urls().next().is_some() {
            return Err(NetError::RelaysNotInstalled);
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
}

const PROBE_MAX: usize = 64 * 1024;

fn check_probe(payload_len: usize, limit: usize) -> Result<(), NetError> {
    if payload_len > PROBE_MAX || limit > PROBE_MAX || limit < payload_len {
        return Err(NetError::Quic("probe is limited to 64 KiB".to_string()));
    }
    Ok(())
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
        };
        assert!(matches!(
            QuicEndpoint::bind(&forbidden, addr, &seed(1)).await,
            Err(NetError::ForbiddenRelay { .. })
        ));
        let operator = EndpointConfig {
            relays: vec!["https://relay.example.test:443".to_string()],
            use_n0_defaults: false,
        };
        assert_eq!(
            QuicEndpoint::bind(&operator, addr, &seed(1))
                .await
                .expect_err("relay"),
            NetError::RelaysNotInstalled
        );
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
}

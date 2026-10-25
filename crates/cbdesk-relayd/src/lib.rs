// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! A loopback iroh relay.
//!
//! The process binds `127.0.0.1` or `::1` only, presents a self-signed
//! certificate, and prints an `https://` URL. CBDesk clients skip certificate
//! checks only when every configured relay host is a literal loopback
//! address. n0 relay defaults are not installed. A successful loopback echo
//! is not a symmetric-NAT measurement and not a published image.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use iroh_relay::server::{CertConfig, QuicConfig, RelayConfig, Server, ServerConfig, TlsConfig};

/// A relay that is listening on loopback.
pub struct LoopbackRelay {
    server: Server,
    url: String,
}

impl LoopbackRelay {
    /// `https://127.0.0.1:port` or `https://[::1]:port`.
    pub fn https_url(&self) -> &str {
        &self.url
    }

    /// Stop the listeners.
    pub async fn shutdown(self) -> Result<(), RelayError> {
        self.server
            .shutdown()
            .await
            .map_err(|err| RelayError::Shutdown(err.to_string()))
    }
}

/// Start a relay on `bind` with an ephemeral HTTPS port.
///
/// `bind` must be a loopback address. The plain HTTP captive-portal port and
/// the QUIC port are also ephemeral and also loopback. The certificate is
/// generated in this process and is not from a public authority.
pub async fn spawn(bind: IpAddr) -> Result<LoopbackRelay, RelayError> {
    if !bind.is_loopback() {
        return Err(RelayError::NotLoopback);
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let server_config = self_signed_server()?;
    let mut relay = RelayConfig::new(SocketAddr::new(bind, 0));
    relay.tls = Some(TlsConfig::new(
        SocketAddr::new(bind, 0),
        CertConfig::Manual { server_config },
    ));
    relay.key_cache_capacity = Some(1024);
    let quic = QuicConfig::new(SocketAddr::new(bind, 0));
    // `ServerConfig` is non-exhaustive, so the fields are assigned after
    // `Default` rather than in a struct literal.
    let mut config = ServerConfig::default();
    config.relay = Some(relay);
    config.quic = Some(quic);
    config.metrics_addr = None;
    let server = Server::spawn(config)
        .await
        .map_err(|err| RelayError::Bind(err.to_string()))?;
    let https = server.https_addr().ok_or(RelayError::NoHttps)?;
    if !https.ip().is_loopback() {
        let _ = server.shutdown().await;
        return Err(RelayError::NotLoopback);
    }
    let url = format!("https://{https}");
    Ok(LoopbackRelay { server, url })
}

/// Default bind address: IPv4 loopback.
pub fn default_bind() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

/// Read `--bind <loopback-ip>` or no arguments.
pub fn parse_bind(args: &[String]) -> Result<IpAddr, RelayError> {
    match args {
        [] => Ok(default_bind()),
        [flag] if flag == "--help" || flag == "-h" => Err(RelayError::Help),
        [flag, value] if flag == "--bind" => {
            let ip: IpAddr = value.parse().map_err(|_| RelayError::BadBind)?;
            if !ip.is_loopback() {
                return Err(RelayError::NotLoopback);
            }
            Ok(ip)
        }
        _ => Err(RelayError::Usage),
    }
}

fn self_signed_server() -> Result<rustls::ServerConfig, RelayError> {
    let cert = rcgen::generate_simple_self_signed(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ])
    .map_err(|err| RelayError::Certificate(err.to_string()))?;
    let rustls_cert = cert.cert.der().clone();
    let private_key = rustls::pki_types::PrivateKeyDer::from(
        rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()),
    );
    rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|err| RelayError::Certificate(err.to_string()))?
        .with_no_client_auth()
        .with_single_cert(vec![rustls_cert], private_key)
        .map_err(|err| RelayError::Certificate(err.to_string()))
}

/// Why the relay did not stay up.
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    /// The address was not a literal loopback IP.
    #[error("refuses a non-loopback bind; this build is not a public relay")]
    NotLoopback,
    /// `--bind` was not an IP address.
    #[error("`--bind` must be a literal loopback IP address")]
    BadBind,
    /// The arguments were not recognized.
    #[error("usage: cbdesk-relayd [--bind 127.0.0.1]")]
    Usage,
    /// `--help` was requested.
    #[error(
        "cbdesk-relayd binds a loopback iroh relay and prints its https URL.\n\
         The certificate is self-signed. A non-loopback address is refused.\n\
         This is not a NAT measurement and not a published image.\n\
         usage: cbdesk-relayd [--bind 127.0.0.1]"
    )]
    Help,
    /// The self-signed certificate could not be built.
    #[error("self-signed certificate: {0}")]
    Certificate(String),
    /// The listener did not bind.
    #[error("relay bind: {0}")]
    Bind(String),
    /// HTTPS did not report an address.
    #[error("relay did not bind an https address")]
    NoHttps,
    /// Shutdown failed.
    #[error("relay shutdown: {0}")]
    Shutdown(String),
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use cbdesk_crypto::Identity;
    use cbdesk_net::{dial_verified_ticket, EndpointConfig, QuicEndpoint};
    use cbdesk_proto::{InvitePurpose, InviteTicket};

    use super::*;

    #[test]
    fn a_public_address_is_refused_before_a_socket_is_opened() {
        let err =
            parse_bind(&["--bind".to_string(), "203.0.113.9".to_string()]).expect_err("public");
        assert!(matches!(err, RelayError::NotLoopback));
        assert!(parse_bind(&[]).expect("default").is_loopback());
        assert!(parse_bind(&["--bind".to_string(), "127.0.0.1".to_string()])
            .expect("v4")
            .is_loopback());
        assert!(parse_bind(&["--bind".to_string(), "::1".to_string()])
            .expect("v6")
            .is_loopback());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn loopback_relay_echoes_a_probe_with_udp_off() {
        let relay = spawn(default_bind()).await.expect("relay");
        let text = relay.https_url().to_string();
        assert!(text.starts_with("https://127.0.0.1:"), "{text}");
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
            .expect("host");
        let client = QuicEndpoint::bind(&config, addr, &client_seed)
            .await
            .expect("client");
        let identity = Identity::from_seed(&host_seed).expect("identity");
        let ready = tokio::time::timeout(Duration::from_secs(8), async {
            tokio::join!(host.online(), client.online());
        })
        .await;
        assert!(ready.is_ok(), "loopback relay did not connect");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let ticket = cbdesk_discovery::sign_ticket(
            InviteTicket {
                relay_urls: vec![text],
                expires_at: now.saturating_add(600),
                purpose: InvitePurpose::Session,
                ..InviteTicket::default()
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
            let conn = dial_verified_ticket(&client, &ticket, now, &mut replay)
                .await
                .map_err(|err| cbdesk_net::NetError::Quic(err.to_string()))?;
            conn.probe_send(b"relayd", 64).await
        };
        let (reflected, echoed) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(15), reflect),
            tokio::time::timeout(Duration::from_secs(15), dial),
        );
        let echoed = echoed.expect("echo timed out").expect("echo");
        reflected.expect("reflect timed out").expect("reflect");
        assert_eq!(echoed, b"relayd");
        assert!(host.dns_queries().is_empty(), "{:?}", host.dns_queries());
        assert!(
            client.dns_queries().is_empty(),
            "{:?}",
            client.dns_queries()
        );
        host.close().await;
        client.close().await;
        relay.shutdown().await.expect("shutdown");
    }

    fn seed(byte: u8) -> [u8; 32] {
        let mut seed = [9_u8; 32];
        seed[0] = byte;
        seed
    }
}

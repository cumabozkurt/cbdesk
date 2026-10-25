// SPDX-License-Identifier: AGPL-3.0-or-later

//! Relay endpoint policy.
//!
//! The default endpoint has an empty relay list and `use_n0_defaults` is
//! false. Hosts that match the known n0 suffixes on a DNS label boundary are
//! rejected before a connection is offered. A URL the operator configured,
//! and that is not on that list, can be installed. `RelayMode::Default` is
//! not selected. See ADR-006.

use crate::error::NetError;

/// Suffixes that must not appear as a relay host.
pub const FORBIDDEN_RELAY_SUFFIXES: &[&str] =
    &["iroh.link", "n0.computer", "n0.io", "iroh.network"];

/// How an endpoint is allowed to learn about relays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EndpointConfig {
    /// Relay URLs the operator configured. Empty by default.
    pub relays: Vec<String>,
    /// Must stay false. There is no built-in public relay list.
    pub use_n0_defaults: bool,
    /// Skip the UDP socket and use only the configured relays.
    ///
    /// The default is false: a normal endpoint still binds a direct socket.
    /// A test sets this to prove a path that has no direct addresses.
    pub relay_only: bool,
    /// Stay on the local network: no relay and no DHT publish.
    ///
    /// The default is false. Turning this on with a relay URL is rejected.
    pub isolated: bool,
}

/// A relay the dialer was told about, usually from a ticket or a DHT record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayHint {
    /// `relay://host:port` or a URL with a scheme.
    pub url: String,
}

/// Reject default public relays and any configured relay on a forbidden suffix.
pub fn validate_endpoint(config: &EndpointConfig) -> Result<(), NetError> {
    if config.use_n0_defaults {
        return Err(NetError::N0DefaultsForbidden);
    }
    if config.isolated && (config.relay_only || !config.relays.is_empty()) {
        return Err(NetError::IsolatedRejectsRelay);
    }
    let hints: Vec<RelayHint> = config
        .relays
        .iter()
        .cloned()
        .map(|url| RelayHint { url })
        .collect();
    check_hints(&hints)
}

/// Reject hints whose host matches [`FORBIDDEN_RELAY_SUFFIXES`].
pub fn check_hints(hints: &[RelayHint]) -> Result<(), NetError> {
    for hint in hints {
        if let Some(host) = relay_host(&hint.url) {
            if host_forbidden(&host) {
                return Err(NetError::ForbiddenRelay { host });
            }
        }
    }
    Ok(())
}

/// Host portion of a relay URL, without brackets or a trailing DNS dot.
pub fn relay_host(input: &str) -> Option<String> {
    let without_scheme = input.split_once("://").map_or(input, |(_, rest)| rest);
    let without_path = without_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if without_path.is_empty() {
        return None;
    }
    if let Some(rest) = without_path.strip_prefix('[') {
        let host = rest.split_once(']')?.0;
        if host.is_empty() {
            return None;
        }
        return Some(host.to_string());
    }
    if without_path.matches(':').count() == 1 {
        let host = without_path.split_once(':')?.0;
        if host.is_empty() {
            return None;
        }
        return Some(host.to_string());
    }
    Some(without_path.to_string())
}

/// True when `host` matches a forbidden relay suffix on a label boundary.
pub(crate) fn host_forbidden(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    FORBIDDEN_RELAY_SUFFIXES
        .iter()
        .any(|suffix| label_suffix(&normalized, suffix))
}

fn label_suffix(host: &str, suffix: &str) -> bool {
    host == suffix
        || host
            .strip_suffix(suffix)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_endpoint_is_empty_and_n0_defaults_are_rejected() {
        let config = EndpointConfig::default();
        assert!(config.relays.is_empty());
        assert!(!config.use_n0_defaults);
        validate_endpoint(&config).expect("default");
        let enabled = EndpointConfig {
            relays: Vec::new(),
            use_n0_defaults: true,
            relay_only: false,
            isolated: false,
        };
        assert_eq!(
            validate_endpoint(&enabled),
            Err(NetError::N0DefaultsForbidden)
        );
    }

    #[test]
    fn isolated_mode_rejects_a_relay() {
        let config = EndpointConfig {
            relays: vec!["https://203.0.113.9:443".to_string()],
            isolated: true,
            ..EndpointConfig::default()
        };
        assert_eq!(
            validate_endpoint(&config),
            Err(NetError::IsolatedRejectsRelay)
        );
        let quiet = EndpointConfig {
            isolated: true,
            ..EndpointConfig::default()
        };
        validate_endpoint(&quiet).expect("isolated without a relay");
    }

    #[test]
    fn suffix_match_is_a_label_boundary() {
        let forbidden = [
            "iroh.link",
            "foo.iroh.link",
            "FOO.IROH.LINK.",
            "relay.n0.computer",
            "x.n0.io",
            "y.iroh.network",
            "relay://foo.iroh.link:443",
            "https://bar.n0.computer/path",
        ];
        for input in forbidden {
            let hints = [RelayHint {
                url: input.to_string(),
            }];
            assert!(check_hints(&hints).is_err(), "{input}");
        }
        let allowed = [
            "notiroh.link",
            "evil-iroh.link",
            "relay://203.0.113.9:443",
            "relay://example.test:443",
        ];
        for input in allowed {
            let hints = [RelayHint {
                url: input.to_string(),
            }];
            assert_eq!(check_hints(&hints), Ok(()), "{input}");
        }
    }
}

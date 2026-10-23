// SPDX-License-Identifier: AGPL-3.0-or-later

//! LAN discovery records.
//!
//! mDNS service type `_cbdesk._udp.local.` plus a small UDP broadcast packet
//! for networks that block multicast DNS. A discovered peer is not trusted:
//! authentication is the same as on any other path. Public networks do not
//! announce even when the user left LAN discovery on.

use cbdesk_crypto::DeviceId;

use crate::error::DiscoveryError;

/// UDP port used by the broadcast fallback. It deliberately avoids 21115–21119.
pub const LAN_BROADCAST_PORT: u16 = 22118;

/// DNS-SD service type, including the trailing dot used by most resolvers.
pub const MDNS_SERVICE_TYPE: &str = "_cbdesk._udp.local.";

/// Standard mDNS UDP port. This is not a CBDesk allocation.
pub const MDNS_PORT: u16 = 5353;

/// IPv4 mDNS group.
pub const MDNS_IPV4: std::net::Ipv4Addr = std::net::Ipv4Addr::new(224, 0, 0, 251);

/// Name used when the user has not allowed the real device name to be broadcast.
pub const GENERIC_DEVICE_NAME: &str = "CBDesk device";

const MAGIC: &[u8; 4] = b"CBDS";
const VERSION: u8 = 1;
const MAX_NAME_LEN: usize = 63;

/// How the operating system classifies the current network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkProfile {
    /// Home or work, as reported by the OS.
    Private,
    /// Public Wi-Fi or an equivalent untrusted profile.
    Public,
    /// The OS did not say. Announcements stay enabled if the user asked for them.
    Unknown,
}

/// Whether this device should answer LAN discovery right now.
pub fn lan_should_announce(user_enabled: bool, profile: NetworkProfile) -> bool {
    user_enabled && profile != NetworkProfile::Public
}

/// Broadcast body. `name` is `None` unless the user opted in to publishing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanPacket {
    /// Full device id.
    pub device_id: DeviceId,
    /// QUIC port.
    pub port: u16,
    /// Optional device name.
    pub name: Option<String>,
}

impl LanPacket {
    /// Encode the broadcast body.
    pub fn encode(&self) -> Result<Vec<u8>, DiscoveryError> {
        let name = self.name.as_deref().unwrap_or("");
        if name.len() > MAX_NAME_LEN || !name.is_char_boundary(name.len()) {
            return Err(DiscoveryError::BadLanPacket);
        }
        let mut out = Vec::with_capacity(4 + 1 + 32 + 2 + 1 + name.len());
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(&self.port.to_be_bytes());
        out.push(u8::try_from(name.len()).map_err(|_| DiscoveryError::BadLanPacket)?);
        out.extend_from_slice(name.as_bytes());
        Ok(out)
    }

    /// Decode a broadcast body. Trailing bytes are rejected.
    pub fn decode(bytes: &[u8]) -> Result<Self, DiscoveryError> {
        if bytes.len() < 4 + 1 + 32 + 2 + 1 || bytes[..4] != *MAGIC || bytes[4] != VERSION {
            return Err(DiscoveryError::BadLanPacket);
        }
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes[5..37]);
        let device_id = DeviceId::from_bytes(id)?;
        let port = u16::from_be_bytes([bytes[37], bytes[38]]);
        let name_len = usize::from(bytes[39]);
        let end = 40 + name_len;
        if end != bytes.len() {
            return Err(DiscoveryError::BadLanPacket);
        }
        let name = if name_len == 0 {
            None
        } else {
            let text =
                std::str::from_utf8(&bytes[40..end]).map_err(|_| DiscoveryError::BadLanPacket)?;
            Some(text.to_string())
        };
        Ok(Self {
            device_id,
            port,
            name,
        })
    }
}

/// mDNS TXT fields. `pk` is the first 16 bytes of the public key, in hex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MdnsTxt {
    /// Protocol version, currently 1.
    pub version: u8,
    /// 32 lowercase hex characters.
    pub pk_hex16: String,
    /// QUIC port.
    pub port: u16,
    /// Present only when the user opted in.
    pub name: Option<String>,
}

impl MdnsTxt {
    /// Build TXT fields from a device id.
    pub fn from_device(device_id: &DeviceId, port: u16, name: Option<String>) -> Self {
        Self {
            version: 1,
            pk_hex16: hex16(&device_id.as_bytes()[..16]),
            port,
            name,
        }
    }

    /// TXT pairs in a stable order.
    pub fn pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![
            ("v".to_string(), self.version.to_string()),
            ("pk".to_string(), self.pk_hex16.clone()),
            ("port".to_string(), self.port.to_string()),
        ];
        if let Some(name) = &self.name {
            pairs.push(("name".to_string(), name.clone()));
        }
        pairs
    }

    /// DNS TXT character-strings, `key=value`, in [`Self::pairs`] order.
    pub fn character_strings(&self) -> Vec<String> {
        self.pairs()
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect()
    }

    /// Parse TXT character-strings produced by [`Self::character_strings`].
    ///
    /// Keys other than `v`, `pk`, `port`, and `name` are rejected. `pk` is 32
    /// lowercase hex characters. An empty name is rejected.
    pub fn from_character_strings<I, S>(items: I) -> Result<Self, DiscoveryError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut version = None;
        let mut pk_hex16 = None;
        let mut port = None;
        let mut name = None;
        for item in items {
            let (key, value) = item
                .as_ref()
                .split_once('=')
                .ok_or(DiscoveryError::BadMdns)?;
            match key {
                "v" if version.is_none() => {
                    if value != "1" {
                        return Err(DiscoveryError::BadMdns);
                    }
                    version = Some(1u8);
                }
                "pk" if pk_hex16.is_none() => {
                    if !is_pk_hex(value) {
                        return Err(DiscoveryError::BadMdns);
                    }
                    pk_hex16 = Some(value.to_string());
                }
                "port" if port.is_none() => {
                    port = Some(value.parse().map_err(|_| DiscoveryError::BadMdns)?);
                }
                "name" if name.is_none() => {
                    if value.is_empty() || value.len() > MAX_NAME_LEN {
                        return Err(DiscoveryError::BadMdns);
                    }
                    name = Some(value.to_string());
                }
                _ => return Err(DiscoveryError::BadMdns),
            }
        }
        Ok(Self {
            version: version.ok_or(DiscoveryError::BadMdns)?,
            pk_hex16: pk_hex16.ok_or(DiscoveryError::BadMdns)?,
            port: port.ok_or(DiscoveryError::BadMdns)?,
            name,
        })
    }
}

fn is_pk_hex(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex16(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_crypto::Identity;

    #[test]
    fn public_networks_suppress_announcements() {
        assert!(lan_should_announce(true, NetworkProfile::Private));
        assert!(lan_should_announce(true, NetworkProfile::Unknown));
        assert!(!lan_should_announce(true, NetworkProfile::Public));
        assert!(!lan_should_announce(false, NetworkProfile::Private));
        assert_eq!(LAN_BROADCAST_PORT, 22118);
        assert!(!(21115..=21119).contains(&LAN_BROADCAST_PORT));
        assert_eq!(MDNS_PORT, 5353);
        assert_eq!(MDNS_IPV4, std::net::Ipv4Addr::new(224, 0, 0, 251));
    }

    #[test]
    fn packet_and_txt_roundtrip_without_a_name_by_default() {
        let id = Identity::from_seed(&[5u8; 32]).expect("id").device_id();
        let packet = LanPacket {
            device_id: id,
            port: 22116,
            name: None,
        };
        let encoded = packet.encode().expect("encode");
        assert_eq!(LanPacket::decode(&encoded).expect("decode"), packet);
        assert!(LanPacket::decode(&encoded[..encoded.len() - 1]).is_err());

        let txt = MdnsTxt::from_device(&id, 22116, None);
        assert_eq!(txt.pk_hex16.len(), 32);
        assert!(txt.pairs().iter().all(|(key, _)| key != "name"));
        let named = MdnsTxt::from_device(&id, 22116, Some("Ofis".to_string()));
        assert!(named
            .pairs()
            .iter()
            .any(|(key, value)| key == "name" && value == "Ofis"));
        assert_eq!(
            MdnsTxt::from_character_strings(named.character_strings()).expect("txt"),
            named
        );
        assert_eq!(
            MdnsTxt::from_character_strings(txt.character_strings()).expect("txt"),
            txt
        );
        assert!(MdnsTxt::from_character_strings(["v=1", "pk=abcd", "port=1"]).is_err());
        assert!(MdnsTxt::from_character_strings([
            "v=2",
            "pk=00112233445566778899aabbccddeeff",
            "port=1"
        ])
        .is_err());
    }
}

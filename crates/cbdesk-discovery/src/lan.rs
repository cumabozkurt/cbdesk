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
    }
}

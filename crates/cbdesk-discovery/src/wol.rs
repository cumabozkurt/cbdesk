// SPDX-License-Identifier: AGPL-3.0-or-later

//! Wake-on-LAN magic packet bytes.
//!
//! [`magic_packet`] builds the 102-byte payload. It does not open a socket
//! and it does not transmit the packet. A caller that sends it does so on a
//! network the operator already chose.

use crate::DiscoveryError;

/// Build a Wake-on-LAN payload for `mac`.
///
/// The payload is six `0xff` bytes followed by the MAC address repeated 16
/// times. A multicast or broadcast MAC is refused so this helper is not a
/// directed broadcast by accident. The all-zero MAC is also refused.
pub fn magic_packet(mac: [u8; 6]) -> Result<[u8; 102], DiscoveryError> {
    if mac == [0_u8; 6] || mac[0] & 0x01 == 0x01 {
        return Err(DiscoveryError::BadLanPacket);
    }
    let mut packet = [0_u8; 102];
    packet[..6].fill(0xff);
    for copy in 0..16 {
        let start = 6 + copy * 6;
        packet[start..start + 6].copy_from_slice(&mac);
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_magic_packet_repeats_the_mac_and_is_not_sent() {
        let mac = [0x02, 0x00, 0x00, 0x00, 0x00, 0x11];
        let packet = magic_packet(mac).expect("packet");
        assert_eq!(&packet[..6], &[0xff; 6]);
        assert_eq!(&packet[6..12], &mac);
        assert_eq!(&packet[96..102], &mac);
        assert!(magic_packet([0xff; 6]).is_err());
        assert!(magic_packet([0; 6]).is_err());
    }
}

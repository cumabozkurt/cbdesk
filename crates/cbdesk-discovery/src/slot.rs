// SPDX-License-Identifier: AGPL-3.0-or-later

//! Short-code rendezvous slot keys.
//!
//! `BLAKE3("cbdesk-slot-v1" || slot || floor(unix_time / 600))`. The slot
//! number is public. The epoch changes at every tenth minute, including the
//! boundary between second 599 and second 600.

use crate::error::DiscoveryError;

/// Seconds in one short-code rendezvous epoch.
pub const SLOT_EPOCH_SECS: u64 = 600;

/// Epoch index for a unix timestamp.
pub fn slot_epoch(unix_secs: u64) -> u64 {
    unix_secs / SLOT_EPOCH_SECS
}

/// Derive the DHT slot key. `slot` must be `0..=999`.
pub fn slot_key(slot: u16, unix_secs: u64) -> Result<[u8; 32], DiscoveryError> {
    if slot > 999 {
        return Err(DiscoveryError::SlotOutOfRange);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cbdesk-slot-v1");
    hasher.update(&slot.to_be_bytes());
    hasher.update(&slot_epoch(unix_secs).to_be_bytes());
    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_stable_inside_ten_minutes_and_changes_at_600() {
        let early = slot_key(428, 599).expect("599");
        let still = slot_key(428, 0).expect("0");
        let next = slot_key(428, 600).expect("600");
        let later = slot_key(428, 1_199).expect("1199");
        let after = slot_key(428, 1_200).expect("1200");
        assert_eq!(early, still);
        assert_eq!(next, later);
        assert_ne!(early, next);
        assert_ne!(next, after);
        assert!(slot_key(1_000, 0).is_err());
        assert_ne!(slot_key(1, 0).expect("1"), slot_key(2, 0).expect("2"));
    }
}

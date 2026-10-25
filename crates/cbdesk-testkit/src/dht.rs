// SPDX-License-Identifier: AGPL-3.0-or-later

//! A DHT stand-in that stores signed address records in a map.
//!
//! Poison mode flips one byte of the stored record so verification fails.
//! Nothing is published to a network.

use std::collections::HashMap;

use cbdesk_crypto::{DeviceId, Identity};
use cbdesk_discovery::{sign_dht_record, verify_dht_record, DhtKeyMode};
use cbdesk_proto::DhtAddressRecord;

use crate::error::TestkitError;

/// In-memory signed address records.
#[derive(Clone, Debug, Default)]
pub struct FakeDht {
    poison: bool,
    records: HashMap<DeviceId, Vec<u8>>,
    signatures: HashMap<DeviceId, Vec<u8>>,
    signers: HashMap<DeviceId, Vec<u8>>,
}

impl FakeDht {
    /// An empty DHT.
    pub fn new() -> Self {
        Self::default()
    }

    /// Flip a stored record byte on insert.
    pub fn poison(mut self) -> Self {
        self.poison = true;
        self
    }

    /// Sign `record` with the long-term key and store it.
    pub fn insert(
        &mut self,
        identity: &Identity,
        record: &DhtAddressRecord,
    ) -> Result<(), TestkitError> {
        let mut signed = sign_dht_record(record, identity, DhtKeyMode::LongTerm)?;
        if self.poison {
            if signed.record.is_empty() {
                signed.record.push(0xff);
            } else {
                signed.record[0] ^= 0xff;
            }
        }
        let id = identity.device_id();
        self.signers.insert(id, signed.signer_pk);
        self.signatures.insert(id, signed.sig);
        self.records.insert(id, signed.record);
        Ok(())
    }

    /// Verify and decode the record stored for `id`.
    pub fn get(&self, id: &DeviceId) -> Result<DhtAddressRecord, TestkitError> {
        let signed = cbdesk_proto::SignedDhtRecord {
            signer_pk: self.signers.get(id).cloned().unwrap_or_default(),
            record: self.records.get(id).cloned().unwrap_or_default(),
            sig: self.signatures.get(id).cloned().unwrap_or_default(),
            unknown: cbdesk_proto::UnknownFields::new(),
        };
        let (_id, record) = verify_dht_record(&signed)?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_proto::UnknownFields;

    #[test]
    fn poison_makes_verification_fail() {
        let identity = Identity::from_seed(&[6_u8; 32]).expect("id");
        let record = DhtAddressRecord {
            direct_addrs: vec!["203.0.113.8:22116".to_string()],
            relay_urls: Vec::new(),
            protocol_version: 1,
            timestamp: 1_700_000_000,
            publish_ip: true,
            unknown: UnknownFields::new(),
        };
        let mut honest = FakeDht::new();
        honest.insert(&identity, &record).expect("insert");
        assert_eq!(honest.get(&identity.device_id()).expect("get"), record);
        let mut poisoned = FakeDht::new().poison();
        poisoned.insert(&identity, &record).expect("insert");
        assert!(poisoned.get(&identity.device_id()).is_err());
    }
}

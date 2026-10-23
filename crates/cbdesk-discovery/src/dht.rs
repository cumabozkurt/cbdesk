// SPDX-License-Identifier: AGPL-3.0-or-later

//! Signed DHT address records.
//!
//! Blinded publishing is rejected. The HKDF epoch seed in `cbdesk-crypto` is
//! covered by tests, but it cannot be derived from the public key, so it does
//! not meet the unlinkability design. ADR-007 tracks that gap.

use cbdesk_crypto::{DeviceId, Identity};
use cbdesk_proto::{
    DhtAddressRecord, ProtoMessage, SignedDhtRecord, UnknownFields, MAX_DIRECT_ADDRS,
};

use crate::error::DiscoveryError;

/// How the publisher chooses the key that signs a DHT record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DhtKeyMode {
    /// Sign with the long-term device key. This is the only enabled mode.
    LongTerm,
    /// Reserved. Returns [`DiscoveryError::BlindingPendingReview`].
    Blinded,
}

/// Sign an address record. Blinded mode fails closed.
pub fn sign_dht_record(
    record: &DhtAddressRecord,
    identity: &Identity,
    mode: DhtKeyMode,
) -> Result<SignedDhtRecord, DiscoveryError> {
    if mode == DhtKeyMode::Blinded {
        return Err(DiscoveryError::BlindingPendingReview);
    }
    validate(record)?;
    let bytes = record.to_bytes()?;
    Ok(SignedDhtRecord {
        signer_pk: identity.device_id().as_bytes().to_vec(),
        sig: identity.sign(&bytes).to_vec(),
        record: bytes,
        unknown: UnknownFields::new(),
    })
}

/// Verify the signature and the publish-IP rule.
pub fn verify_dht_record(
    signed: &SignedDhtRecord,
) -> Result<(DeviceId, DhtAddressRecord), DiscoveryError> {
    let bytes: [u8; 32] = signed
        .signer_pk
        .as_slice()
        .try_into()
        .map_err(|_| DiscoveryError::BadDhtRecord)?;
    let device_id = DeviceId::from_bytes(bytes)?;
    device_id.verify(&signed.record, &signed.sig)?;
    let record =
        DhtAddressRecord::from_bytes(&signed.record).map_err(|_| DiscoveryError::BadDhtRecord)?;
    validate(&record)?;
    Ok((device_id, record))
}

fn validate(record: &DhtAddressRecord) -> Result<(), DiscoveryError> {
    if record.direct_addrs.len() > MAX_DIRECT_ADDRS {
        return Err(DiscoveryError::BadDhtRecord);
    }
    if !record.publish_ip && !record.direct_addrs.is_empty() {
        return Err(DiscoveryError::IpNotPublished);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_term_record_verifies_and_blinded_mode_is_rejected() {
        let identity = Identity::from_seed(&[8u8; 32]).expect("id");
        let record = DhtAddressRecord {
            direct_addrs: vec!["203.0.113.8:22116".to_string()],
            relay_urls: vec!["relay://203.0.113.9:443".to_string()],
            protocol_version: 1,
            timestamp: 1_700_000_000,
            publish_ip: true,
            unknown: UnknownFields::new(),
        };
        let signed = sign_dht_record(&record, &identity, DhtKeyMode::LongTerm).expect("sign");
        let (id, decoded) = verify_dht_record(&signed).expect("verify");
        assert_eq!(id, identity.device_id());
        assert_eq!(decoded, record);

        let hidden = DhtAddressRecord {
            publish_ip: false,
            direct_addrs: vec!["203.0.113.8:22116".to_string()],
            ..record.clone()
        };
        assert!(matches!(
            sign_dht_record(&hidden, &identity, DhtKeyMode::LongTerm),
            Err(DiscoveryError::IpNotPublished)
        ));
        assert!(matches!(
            sign_dht_record(&record, &identity, DhtKeyMode::Blinded),
            Err(DiscoveryError::BlindingPendingReview)
        ));
    }

    #[test]
    fn tampered_record_fails_verification() {
        let identity = Identity::from_seed(&[8u8; 32]).expect("id");
        let record = DhtAddressRecord {
            relay_urls: vec!["relay://203.0.113.9:443".to_string()],
            protocol_version: 1,
            timestamp: 10,
            publish_ip: false,
            ..DhtAddressRecord::default()
        };
        let mut signed = sign_dht_record(&record, &identity, DhtKeyMode::LongTerm).expect("sign");
        signed.record[0] ^= 0xff;
        assert!(verify_dht_record(&signed).is_err());
    }
}

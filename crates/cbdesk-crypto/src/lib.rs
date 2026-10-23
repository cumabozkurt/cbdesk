// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Identity keys and the small set of cryptographic helpers CBDesk calls directly.
//!
//! Password-authenticated key exchange and OPAQUE are not implemented here.
//! Short-code PAKE is specified in ADR-008 and belongs to a later milestone,
//! after a cryptographer has reviewed the transcript binding. This crate does
//! not invent a new primitive.

mod error;
mod identity;
mod kdf;
mod keystore;
mod redacted;
mod z32;

pub use error::CryptoError;
pub use identity::{DeviceId, Identity, ShortFingerprint};
pub use kdf::{
    derive_backup_key, dht_epoch_seed, epoch_day, hash_argon2id, x25519_public, x25519_shared,
    BACKUP_MEM_KIB, BACKUP_PARALLELISM, BACKUP_TIME_COST,
};
pub use keystore::{
    identity_from_os, identity_from_store, IdentityStore, MemoryKeyStore, OsKeyStore,
};
pub use redacted::Redacted;

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroize;

    fn hex(input: &str) -> Vec<u8> {
        let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        (0..cleaned.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&cleaned[index..index + 2], 16).expect("hex"))
            .collect()
    }

    fn hex32(input: &str) -> [u8; 32] {
        let bytes = hex(input);
        bytes.as_slice().try_into().expect("32 bytes")
    }

    #[test]
    fn ed25519_rfc8032_test_vector_1() {
        let seed = hex32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let public = hex32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let signature = hex(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155\
             5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        let identity = Identity::from_seed(&seed).expect("seed");
        assert_eq!(identity.device_id().as_bytes(), &public);
        assert_eq!(identity.sign(b"").to_vec(), signature);
        identity
            .device_id()
            .verify(b"", &signature)
            .expect("verify");

        let other = Identity::from_seed(&[2u8; 32]).expect("other");
        assert!(other.device_id().verify(b"", &signature).is_err());
        assert!(identity.device_id().verify(b"no", &signature).is_err());
    }

    #[test]
    fn generated_identity_signs_and_debug_hides_the_seed() {
        let identity = Identity::generate().expect("generate");
        let message = b"cbdesk";
        let signature = identity.sign(message);
        identity
            .device_id()
            .verify(message, &signature)
            .expect("verify");
        let seed = identity.signing_seed();
        let rendered = format!("{identity:?}");
        assert!(!rendered.contains(&hex_encode(&seed)));
        assert!(rendered.contains(&identity.device_id().to_z32()));
        assert_eq!(identity.device_id().to_z32().len(), 52);
        let parsed = DeviceId::from_z32(&identity.device_id().to_z32()).expect("parse");
        assert_eq!(parsed, identity.device_id());
        assert!(identity
            .device_id()
            .permanent_link()
            .starts_with("cbdesk://k/"));
    }

    #[test]
    fn short_fingerprint_is_four_groups_of_fifteen_bits() {
        let identity = Identity::from_seed(&[9u8; 32]).expect("seed");
        let first = identity.device_id().short_fingerprint();
        let second = identity.device_id().short_fingerprint();
        assert_eq!(first, second);
        for group in first.groups() {
            assert!(group <= 0x7fff);
        }
        let text = first.to_string();
        assert_eq!(text.len(), 23);
        assert_eq!(text.chars().filter(|c| *c == '-').count(), 3);
        let other = Identity::from_seed(&[10u8; 32]).expect("other");
        assert_ne!(first, other.device_id().short_fingerprint());
    }

    #[test]
    fn zeroize_wipes_the_seed() {
        let mut identity = Identity::from_seed(&[4u8; 32]).expect("seed");
        identity.zeroize();
        assert_eq!(identity.signing_seed(), [0u8; 32]);
    }

    #[test]
    fn redacted_debug_does_not_show_the_value() {
        let secret = Redacted::new("428-KARTAL-DENIZ".to_string());
        assert_eq!(format!("{secret:?}"), "[REDACTED]");
        assert_eq!(secret.expose(), "428-KARTAL-DENIZ");
    }

    #[test]
    fn blake3_empty_and_abc() {
        let empty = blake3::hash(b"");
        assert_eq!(
            hex_encode(empty.as_bytes()),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        let abc = blake3::hash(b"abc");
        assert_eq!(
            hex_encode(abc.as_bytes()),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn hkdf_rfc5869_case_1_and_epoch_boundary() {
        let ikm = [0x0bu8; 22];
        let salt = hex("000102030405060708090a0b0c");
        let info = hex("f0f1f2f3f4f5f6f7f8f9");
        let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), &ikm);
        let mut okm = [0u8; 42];
        hk.expand(&info, &mut okm).expect("expand");
        assert_eq!(
            hex_encode(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );

        assert_eq!(epoch_day(86_399), 0);
        assert_eq!(epoch_day(86_400), 1);
        let seed = [7u8; 32];
        let day0 = dht_epoch_seed(&seed, 0).expect("day0");
        let day0_again = dht_epoch_seed(&seed, 0).expect("again");
        let day1 = dht_epoch_seed(&seed, 1).expect("day1");
        assert_eq!(day0.as_ref(), day0_again.as_ref());
        assert_ne!(day0.as_ref(), day1.as_ref());
    }

    #[test]
    fn x25519_rfc7748_vector() {
        let alice = hex32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let alice_public =
            hex32("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
        let bob = hex32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        let bob_public = hex32("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
        let shared = hex32("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
        assert_eq!(x25519_public(&alice), alice_public);
        assert_eq!(x25519_public(&bob), bob_public);
        assert_eq!(x25519_shared(&alice, &bob_public).expect("alice"), shared);
        assert_eq!(x25519_shared(&bob, &alice_public).expect("bob"), shared);
    }

    #[test]
    fn argon2id_known_vectors_and_backup_parameters() {
        let small = hash_argon2id(b"password", b"somesalt", 64, 2, 1).expect("small");
        assert_eq!(
            hex_encode(small.as_ref()),
            "16a1a498734609dd01456da406de9f3d9da93e6c86c300a12fc1465214ce4922"
        );
        assert_eq!(BACKUP_MEM_KIB, 64 * 1024);
        assert_eq!(BACKUP_TIME_COST, 3);
        assert_eq!(BACKUP_PARALLELISM, 1);
        let salt: [u8; 16] = core::array::from_fn(|i| i as u8);
        let backup = derive_backup_key(b"cbdesk-backup", &salt).expect("backup");
        assert_eq!(
            hex_encode(backup.as_ref()),
            "d11b2b099a60b1773bb294470af0364265077645df854c7b5588332387e8c04d"
        );
    }

    fn hex_encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}

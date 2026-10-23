// SPDX-License-Identifier: AGPL-3.0-or-later

//! Device identity.
//!
//! A device id is an Ed25519 public key. The short fingerprint is only a
//! display and comparison aid: connections are authenticated with the full key.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use zeroize::Zeroize;

use crate::error::CryptoError;
use crate::z32;

/// First 60 bits of `BLAKE3(public_key)`, shown as four 15-bit groups.
///
/// Each group is at most 32767. The sample `48213-90557-12094-77310` in the
/// master plan is illustrative: values above 32767 cannot occur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortFingerprint([u16; 4]);

impl ShortFingerprint {
    /// The four groups, each in `0..=32767`.
    pub fn groups(self) -> [u16; 4] {
        self.0
    }
}

impl fmt::Display for ShortFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:05}-{:05}-{:05}-{:05}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

/// Ed25519 public key that names a device.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId([u8; 32]);

impl DeviceId {
    /// Borrow the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Build an id from a verified Ed25519 point.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, CryptoError> {
        VerifyingKey::from_bytes(&bytes).map_err(|_| CryptoError::InvalidPublicKey)?;
        Ok(Self(bytes))
    }

    /// Canonical z-base-32 form, 52 characters.
    pub fn to_z32(&self) -> String {
        z32::encode(&self.0).unwrap_or_default()
    }

    /// Parse a canonical z-base-32 device id.
    pub fn from_z32(text: &str) -> Result<Self, CryptoError> {
        Self::from_bytes(z32::decode_device_id(text)?)
    }

    /// Permanent share link. This still does not grant access.
    pub fn permanent_link(&self) -> String {
        format!("cbdesk://k/{}", self.to_z32())
    }

    /// Display fingerprint. Not sufficient on its own to authenticate a peer.
    pub fn short_fingerprint(&self) -> ShortFingerprint {
        let hash = blake3::hash(&self.0);
        let bytes = hash.as_bytes();
        let wide = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let top60 = wide >> 4;
        ShortFingerprint([
            ((top60 >> 45) & 0x7fff) as u16,
            ((top60 >> 30) & 0x7fff) as u16,
            ((top60 >> 15) & 0x7fff) as u16,
            (top60 & 0x7fff) as u16,
        ])
    }

    /// Verify an Ed25519 signature over `message`.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        let key = VerifyingKey::from_bytes(&self.0).map_err(|_| CryptoError::InvalidPublicKey)?;
        let signature = Signature::from_slice(signature).map_err(|_| CryptoError::BadSignature)?;
        key.verify(message, &signature)
            .map_err(|_| CryptoError::BadSignature)
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceId")
            .field("z32", &self.to_z32())
            .finish()
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_z32())
    }
}

/// Secret Ed25519 identity. The seed is wiped when this value is dropped.
pub struct Identity {
    key: SigningKey,
}

impl Identity {
    /// Generate a new identity from the operating system CSPRNG.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).map_err(|_| CryptoError::Rng)?;
        let identity = Self::from_seed(&seed)?;
        seed.zeroize();
        Ok(identity)
    }

    /// Build an identity from a 32-byte seed.
    pub fn from_seed(seed: &[u8; 32]) -> Result<Self, CryptoError> {
        Ok(Self {
            key: SigningKey::from_bytes(seed),
        })
    }

    /// Public device id.
    pub fn device_id(&self) -> DeviceId {
        DeviceId(self.key.verifying_key().to_bytes())
    }

    /// Sign `message`. The signature is not a secret; the seed is.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.key.sign(message).to_bytes()
    }

    /// Copy the seed. The caller has to wipe the copy.
    pub fn signing_seed(&self) -> [u8; 32] {
        self.key.to_bytes()
    }
}

impl Drop for Identity {
    fn drop(&mut self) {
        self.wipe();
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Identity")
            .field("device_id", &self.device_id())
            .finish()
    }
}

impl Zeroize for Identity {
    fn zeroize(&mut self) {
        self.wipe();
    }
}

impl Identity {
    /// Replace the key with an all-zero seed. `SigningKey` does not implement
    /// `Zeroize` in ed25519-dalek 2.1; its `Drop` wipes the previous seed.
    fn wipe(&mut self) {
        self.key = SigningKey::from_bytes(&[0u8; 32]);
    }
}

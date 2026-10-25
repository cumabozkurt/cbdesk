// SPDX-License-Identifier: AGPL-3.0-or-later

//! Key derivation used by backups and the experimental DHT epoch seed.
//!
//! The epoch seed is deterministic and is derived from the long-term secret.
//! It does **not** implement Tor-style public-key blinding. Publishing under a
//! blinded key stays disabled until that construction has a reviewed design.
//! See ADR-007.

use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use crate::CryptoError;

/// Argon2id memory for an identity backup, in kibibytes (64 MiB).
pub const BACKUP_MEM_KIB: u32 = 64 * 1024;

/// Argon2id passes for an identity backup.
pub const BACKUP_TIME_COST: u32 = 3;

/// Argon2id lanes for an identity backup.
pub const BACKUP_PARALLELISM: u32 = 1;

/// Unix day number used as the DHT epoch.
pub fn epoch_day(unix_secs: u64) -> u64 {
    unix_secs / 86_400
}

/// HKDF-SHA256 epoch seed: `HKDF(identity_seed, "cbdesk-dht-v1" || epoch_day)`.
///
/// Only the holder of the long-term seed can compute this. Observers who know
/// the public key cannot. Do not treat it as the blinded DHT key from the plan.
pub fn dht_epoch_seed(
    identity_seed: &[u8; 32],
    epoch_day: u64,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let hk = Hkdf::<Sha256>::new(None, identity_seed);
    let mut info = b"cbdesk-dht-v1".to_vec();
    info.extend_from_slice(&epoch_day.to_le_bytes());
    let mut okm = [0u8; 32];
    hk.expand(&info, &mut okm).map_err(|_| CryptoError::Kdf)?;
    info.zeroize();
    Ok(Zeroizing::new(okm))
}

/// Derive the 32-byte key that would encrypt an identity backup.
pub fn derive_backup_key(
    password: &[u8],
    salt: &[u8; 16],
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    hash_argon2id(
        password,
        salt,
        BACKUP_MEM_KIB,
        BACKUP_TIME_COST,
        BACKUP_PARALLELISM,
    )
}

/// Argon2id (version 0x13) into a 32-byte key.
pub fn hash_argon2id(
    password: &[u8],
    salt: &[u8],
    mem_kib: u32,
    time_cost: u32,
    lanes: u32,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let params = Params::new(mem_kib, time_cost, lanes, Some(32)).map_err(|_| CryptoError::Kdf)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(|_| CryptoError::Kdf)?;
    Ok(Zeroizing::new(out))
}

/// X25519 shared secret. The all-zero output is rejected.
pub fn x25519_shared(secret: &[u8; 32], peer_public: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let secret = x25519_dalek::StaticSecret::from(*secret);
    let peer = x25519_dalek::PublicKey::from(*peer_public);
    let shared = secret.diffie_hellman(&peer);
    let bytes = *shared.as_bytes();
    if bytes == [0u8; 32] {
        return Err(CryptoError::WeakSharedSecret);
    }
    Ok(bytes)
}

/// X25519 public key corresponding to `secret`.
pub fn x25519_public(secret: &[u8; 32]) -> [u8; 32] {
    let secret = x25519_dalek::StaticSecret::from(*secret);
    *x25519_dalek::PublicKey::from(&secret).as_bytes()
}

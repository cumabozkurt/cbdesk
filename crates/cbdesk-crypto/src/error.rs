// SPDX-License-Identifier: AGPL-3.0-or-later

//! Errors from identity and key-derivation helpers.

use thiserror::Error;

/// A cryptographic operation failed, or its input was not usable.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// The operating system random source failed.
    #[error("operating system random source failed")]
    Rng,
    /// A public key was not a valid Ed25519 point.
    #[error("public key is not a valid Ed25519 point")]
    InvalidPublicKey,
    /// A signature was the wrong length or did not verify.
    #[error("signature verification failed")]
    BadSignature,
    /// A z-base-32 string was not a canonical 32-byte device id.
    #[error("device id encoding is invalid")]
    BadDeviceId,
    /// Key derivation failed.
    #[error("key derivation failed")]
    Kdf,
    /// An X25519 shared secret was the all-zero value and was rejected.
    #[error("X25519 shared secret was all zeros")]
    WeakSharedSecret,
    /// SPAKE2 pairing failed. The password is not included.
    #[error("pairing failed")]
    Pairing,
    /// The operating-system credential store could not be used.
    ///
    /// The detail is omitted on purpose: some store errors attach the secret.
    #[error("operating system keychain is unavailable")]
    KeyStoreUnavailable,
}

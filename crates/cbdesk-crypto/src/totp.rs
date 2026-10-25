// SPDX-License-Identifier: AGPL-3.0-or-later

//! TOTP (RFC 6238) with HMAC-SHA1.
//!
//! The code is the six-digit value from RFC 6238's SHA-1 test vector. This
//! module does not store a secret and does not invent a MAC. SHA-256 and
//! SHA-512 variants from the RFC are not implemented.

use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::CryptoError;

type HmacSha1 = Hmac<Sha1>;

/// Step length used by the common authenticator apps, in seconds.
pub const TOTP_STEP_SECS: u64 = 30;

/// Digits in the code this crate checks.
pub const TOTP_DIGITS: u32 = 6;

/// Compute a TOTP code for `unix_secs`.
///
/// `secret` is the raw key, not a base32 string. `step` must be non-zero.
/// `digits` must be from 6 through 8.
pub fn totp(secret: &[u8], unix_secs: u64, step: u64, digits: u32) -> Result<u32, CryptoError> {
    if secret.is_empty() || step == 0 || !(6..=8).contains(&digits) {
        return Err(CryptoError::PasswordProtocol);
    }
    let counter = unix_secs / step;
    let mut mac = HmacSha1::new_from_slice(secret).map_err(|_| CryptoError::PasswordProtocol)?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let binary = u32::from_be_bytes([
        digest[offset] & 0x7f,
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    let modulus = 10_u32.pow(digits);
    Ok(binary % modulus)
}

/// Compare two codes of the same width without stopping at the first digit.
pub fn codes_match(left: u32, right: u32) -> bool {
    left.to_ne_bytes()
        .iter()
        .zip(right.to_ne_bytes())
        .fold(0, |diff, (one, other)| diff | (one ^ other))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_sha1_vectors() {
        let secret = b"12345678901234567890";
        let cases = [
            (59_u64, 94287082_u32),
            (1_111_111_109, 7081804),
            (1_111_111_111, 14050471),
            (1_234_567_890, 89005924),
            (2_000_000_000, 69279037),
            (20_000_000_000, 65353130),
        ];
        for (when, expected) in cases {
            let code = totp(secret, when, TOTP_STEP_SECS, 8).expect("totp");
            assert_eq!(code, expected, "time {when}");
        }
        let six = totp(secret, 59, TOTP_STEP_SECS, 6).expect("six");
        assert_eq!(six, 287082);
        assert!(codes_match(six, 287082));
        assert!(!codes_match(six, 287083));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Byte compare for the damage scan.
//!
//! Sixteen-byte groups use [`wide::u8x16`]. On `x86_64` that type is SSE2.
//! On `aarch64` it is NEON. Both are baseline features of those targets, so
//! this is not an AVX or a runtime-dispatched kernel. A shorter tail is
//! scalar. This crate still has no `unsafe` block. The intrinsics live in
//! `wide` 1.7.1.

use wide::u8x16;

/// True when `left` and `right` are different lengths or any byte differs.
pub(crate) fn bytes_differ(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return true;
    }
    let mut offset = 0;
    while offset + 16 <= left.len() {
        let mut a = [0_u8; 16];
        let mut b = [0_u8; 16];
        a.copy_from_slice(&left[offset..offset + 16]);
        b.copy_from_slice(&right[offset..offset + 16]);
        if u8x16::new(a) != u8x16::new(b) {
            return true;
        }
        offset += 16;
    }
    left[offset..] != right[offset..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(left: &[u8], right: &[u8]) -> bool {
        left != right
    }

    #[test]
    fn simd_compare_matches_a_byte_walk() {
        let samples: &[&[u8]] = &[
            &[],
            &[1],
            &[0_u8; 15],
            &[0_u8; 16],
            &[0_u8; 17],
            &[0_u8; 31],
            &[0_u8; 32],
            &[7_u8; 64],
        ];
        for left in samples {
            assert!(!bytes_differ(left, left));
            assert_eq!(bytes_differ(left, left), scalar(left, left));
            if left.is_empty() {
                continue;
            }
            for index in [0, left.len() / 2, left.len() - 1] {
                let mut right = left.to_vec();
                right[index] ^= 1;
                assert!(bytes_differ(left, &right));
                assert_eq!(bytes_differ(left, &right), scalar(left, &right));
            }
        }
        assert!(bytes_differ(&[1, 2], &[1]));
    }
}

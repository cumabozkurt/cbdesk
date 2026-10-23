// SPDX-License-Identifier: AGPL-3.0-or-later

//! z-base-32, the encoding used for device ids.
//!
//! The alphabet and bit packing follow the human-oriented base-32 encoding
//! used by iroh endpoint ids. Device ids additionally require a canonical
//! 52-character form so one key has one string.

use crate::CryptoError;

const ALPHABET: &[u8; 32] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

const INDEX: [Option<u8>; 256] = {
    let mut table = [None; 256];
    let mut i = 0;
    while i < ALPHABET.len() {
        table[ALPHABET[i] as usize] = Some(i as u8);
        i += 1;
    }
    table
};

/// Characters in a canonical 32-byte device id.
pub const DEVICE_ID_LEN: usize = 52;

/// Encode bytes as z-base-32.
pub fn encode(buf: &[u8]) -> Result<String, CryptoError> {
    let bits = buf.len().saturating_mul(8);
    let capacity = if bits % 5 == 0 {
        bits / 5
    } else {
        bits / 5 + 1
    };
    let mut out = Vec::with_capacity(capacity);
    let mut position = 0;
    while position < bits {
        let index = position >> 3;
        let offset = position & 7;
        let value = if offset <= 3 {
            (buf[index] >> (3 - offset)) & 0b1_1111
        } else {
            let shift = offset - 3;
            let high = (buf[index] << shift) & 0b1_1111;
            let low = if index + 1 >= buf.len() {
                0
            } else {
                buf[index + 1] >> (8 - shift)
            };
            high | low
        };
        out.push(ALPHABET[value as usize]);
        position += 5;
    }
    String::from_utf8(out).map_err(|_| CryptoError::BadDeviceId)
}

/// Decode a z-base-32 string into the bytes it represents.
pub fn decode(text: &str) -> Result<Vec<u8>, CryptoError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = vec![0u8; (bytes.len().saturating_mul(5).saturating_add(7)) / 8];
    let remainder = bytes.len() & 7;
    let groups = (bytes.len() - remainder) / 8;
    let mut out_index = 0;
    let mut in_index = 0;

    for _ in 0..groups {
        let a = quintet(bytes, in_index)?;
        let b = quintet(bytes, in_index + 1)?;
        let c = quintet(bytes, in_index + 2)?;
        let d = quintet(bytes, in_index + 3)?;
        let e = quintet(bytes, in_index + 4)?;
        let f = quintet(bytes, in_index + 5)?;
        let g = quintet(bytes, in_index + 6)?;
        let h = quintet(bytes, in_index + 7)?;
        put(&mut out, out_index, (a << 3) | (b >> 2))?;
        put(
            &mut out,
            out_index + 1,
            ((b & 0b11) << 6) | (c << 1) | (d >> 4),
        )?;
        put(&mut out, out_index + 2, ((d & 0b1111) << 4) | (e >> 1))?;
        put(
            &mut out,
            out_index + 3,
            ((e & 1) << 7) | (f << 2) | (g >> 3),
        )?;
        put(&mut out, out_index + 4, ((g & 0b111) << 5) | h)?;
        out_index += 5;
        in_index += 8;
    }

    if remainder == 0 {
        return Ok(out[..out_index].to_vec());
    }

    let a = quintet(bytes, in_index)?;
    let b = quintet(bytes, in_index + 1)?;
    put(&mut out, out_index, (a << 3) | (b >> 2))?;
    if remainder <= 2 {
        return Ok(out[..out_index + 1].to_vec());
    }

    let c = quintet(bytes, in_index + 2)?;
    let d = quintet(bytes, in_index + 3)?;
    put(
        &mut out,
        out_index + 1,
        ((b & 0b11) << 6) | (c << 1) | (d >> 4),
    )?;
    if remainder <= 4 {
        return Ok(out[..out_index + 2].to_vec());
    }

    let e = quintet(bytes, in_index + 4)?;
    put(&mut out, out_index + 2, ((d & 0b1111) << 4) | (e >> 1))?;
    if remainder <= 5 {
        return Ok(out[..out_index + 3].to_vec());
    }

    let f = quintet(bytes, in_index + 5)?;
    let g = quintet(bytes, in_index + 6)?;
    put(
        &mut out,
        out_index + 3,
        ((e & 1) << 7) | (f << 2) | (g >> 3),
    )?;
    if remainder <= 7 {
        return Ok(out[..out_index + 4].to_vec());
    }

    let h = quintet(bytes, in_index + 7)?;
    put(&mut out, out_index + 4, ((g & 0b111) << 5) | h)?;
    Ok(out[..out_index + 5].to_vec())
}

/// Decode a canonical 32-byte device id.
pub fn decode_device_id(text: &str) -> Result<[u8; 32], CryptoError> {
    if text.len() != DEVICE_ID_LEN {
        return Err(CryptoError::BadDeviceId);
    }
    let bytes = decode(text)?;
    let array: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::BadDeviceId)?;
    let again = encode(&array)?;
    if again != text {
        return Err(CryptoError::BadDeviceId);
    }
    Ok(array)
}

fn quintet(bytes: &[u8], position: usize) -> Result<u8, CryptoError> {
    if position >= bytes.len() {
        return Ok(0);
    }
    INDEX[bytes[position] as usize].ok_or(CryptoError::BadDeviceId)
}

fn put(out: &mut [u8], index: usize, value: u8) -> Result<(), CryptoError> {
    let slot = out.get_mut(index).ok_or(CryptoError::BadDeviceId)?;
    *slot = value;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_vectors() {
        let cases: &[(&[u8], &str)] = &[
            (b"", ""),
            (&[0], "yy"),
            (&[248], "9y"),
            (&[100, 22], "comy"),
            (&[7], "yh"),
            (&[240, 191, 199], "6n9hq"),
            (&[212, 122, 4], "4t7ye"),
            (b"Hello, World!", "jb1sa5dxfoofq551pt1nn"),
        ];
        for (bytes, encoded) in cases {
            assert_eq!(encode(bytes).expect("encode"), *encoded);
            assert_eq!(decode(encoded).expect("decode"), bytes.to_vec());
        }
    }

    #[test]
    fn thirty_two_bytes_are_fifty_two_characters() {
        let bytes = [0xab; 32];
        let encoded = encode(&bytes).expect("encode");
        assert_eq!(encoded.len(), DEVICE_ID_LEN);
        assert_eq!(decode_device_id(&encoded).expect("id"), bytes);
        assert!(decode_device_id(&encoded.to_uppercase()).is_err());
        let mut mutated = encoded;
        // Flip a character to a value outside the alphabet.
        mutated.replace_range(0..1, "l");
        assert!(decode_device_id(&mutated).is_err());
    }
}

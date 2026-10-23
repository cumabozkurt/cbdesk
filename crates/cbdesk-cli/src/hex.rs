// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hex for the invite ticket text. This is not a cryptographic construction.

use crate::error::CliError;

/// Lowercase hex.
pub fn encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Decode lowercase or uppercase hex.
pub fn decode(text: &str) -> Result<Vec<u8>, CliError> {
    if !text.len().is_multiple_of(2) {
        return Err(CliError::new("ticket hex has an odd length"));
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let high = nibble(bytes[index])?;
        let low = nibble(bytes[index + 1])?;
        out.push((high << 4) | low);
        index += 2;
    }
    Ok(out)
}

fn nibble(byte: u8) -> Result<u8, CliError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(CliError::new("ticket hex has a non-hex digit")),
    }
}

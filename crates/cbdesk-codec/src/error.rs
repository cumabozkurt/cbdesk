// SPDX-License-Identifier: AGPL-3.0-or-later

//! Encode and decode failures.

use thiserror::Error;

/// The software codec could not produce or read a picture.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// Width or height is not an even size the encoder accepts.
    #[error("frame dimensions are not encodable")]
    BadDimensions,
    /// The encoder rejected the picture.
    #[error("encode failed: {0}")]
    Encode(String),
    /// The decoder rejected the payload.
    #[error("decode failed: {0}")]
    Decode(String),
    /// The payload did not yield a picture.
    #[error("payload contained no picture")]
    NoFrame,
}

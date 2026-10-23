// SPDX-License-Identifier: AGPL-3.0-or-later

//! Errors from encoding and decoding protocol messages.

use thiserror::Error;

/// A protocol message could not be encoded or decoded.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtoError {
    /// The buffer is longer than the limit for this message class.
    #[error("message is {len} bytes, above the {limit} byte limit")]
    TooLarge {
        /// Actual length in bytes.
        len: usize,
        /// Allowed maximum in bytes.
        limit: usize,
    },
    /// The buffer ended before a field was complete.
    #[error("truncated protobuf field")]
    Truncated,
    /// A varint was longer than the protobuf maximum, or it overflowed.
    #[error("invalid protobuf varint")]
    BadVarint,
    /// The wire type is not one this codec accepts.
    #[error("invalid protobuf wire type {0}")]
    BadWireType(u8),
    /// Field numbers start at 1.
    #[error("invalid protobuf field number")]
    BadFieldNumber,
    /// A known field arrived with a wire type it does not use.
    #[error("field {0} has the wrong wire type")]
    WrongWireType(u32),
    /// A length-delimited field was not valid UTF-8.
    #[error("string field was not valid UTF-8")]
    BadUtf8,
    /// An integer did not fit in the declared protobuf type.
    #[error("integer field did not fit in its declared type")]
    Overflow,
    /// A floating-point field was NaN or infinite.
    #[error("floating-point field must be finite")]
    NonFinite,
    /// A control message did not contain exactly one body.
    #[error("control message must contain exactly one body")]
    ControlBody,
}

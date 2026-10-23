// SPDX-License-Identifier: AGPL-3.0-or-later

//! Errors from synthetic frames and the fake DHT.

use thiserror::Error;

use cbdesk_discovery::DiscoveryError;

/// A test double rejected its input.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestkitError {
    /// The frame is narrower than the barcode.
    #[error("frame is too narrow for the barcode")]
    FrameTooNarrow,
    /// Width or height was zero, or the product overflowed.
    #[error("frame dimensions are unusable")]
    BadDimensions,
    /// The pixel buffer does not match the claimed size.
    #[error("frame buffer has the wrong length")]
    ShortFrame,
    /// The barcode magic byte was not present.
    #[error("frame barcode is invalid")]
    BadBarcode,
    /// The fake DHT could not sign or verify a record.
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Failures from a capturer. None of these hide a session.

use thiserror::Error;

/// A capturer rejected a frame or could not see a display.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
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
    /// No display, or the desktop portal refused the request.
    #[error("screen capture is unavailable: {0}")]
    Unavailable(String),
}

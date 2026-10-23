// SPDX-License-Identifier: AGPL-3.0-or-later

//! A capturer that paints a barcode instead of a desktop.
//!
//! The barcode is 136 pixels: 8 bits of magic, 64 bits of frame number, and
//! 64 bits of timestamp. Tests read those bits back. This is not a screen
//! grabber and it never captures a real display.

use crate::error::CaptureError;

/// Pixel width of the barcode. Shorter frames are rejected.
pub const BARCODE_WIDTH: u32 = 136;

const MAGIC: u8 = 0xCB;

/// One grayscale frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major grayscale samples, one byte per pixel.
    pub pixels: Vec<u8>,
}

/// Produces barcode frames of a fixed size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntheticCapturer {
    width: u32,
    height: u32,
}

impl SyntheticCapturer {
    /// A capturer that emits `width` by `height` frames.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Paint `frame_no` and `timestamp_us` into the barcode.
    pub fn capture(&self, frame_no: u64, timestamp_us: u64) -> Result<Frame, CaptureError> {
        if self.width < BARCODE_WIDTH || self.height == 0 {
            return Err(if self.height == 0 || self.width == 0 {
                CaptureError::BadDimensions
            } else {
                CaptureError::FrameTooNarrow
            });
        }
        let len = pixel_len(self.width, self.height)?;
        let mut pixels = vec![0_u8; len];
        write_bits(&mut pixels, 0, u64::from(MAGIC), 8);
        write_bits(&mut pixels, 8, frame_no, 64);
        write_bits(&mut pixels, 72, timestamp_us, 64);
        // Repeat the barcode down the frame. A 4x4 block codec keeps a
        // one-pixel-tall stripe only when every row in the block matches.
        let width = self.width as usize;
        for row in 1..self.height as usize {
            let start = row.saturating_mul(width);
            pixels.copy_within(0..width, start);
        }
        Ok(Frame {
            width: self.width,
            height: self.height,
            pixels,
        })
    }
}

/// Read the barcode painted by [`SyntheticCapturer::capture`].
pub fn read_barcode(frame: &Frame) -> Result<(u64, u64), CaptureError> {
    let len = pixel_len(frame.width, frame.height)?;
    if frame.pixels.len() != len || frame.width < BARCODE_WIDTH {
        return Err(CaptureError::ShortFrame);
    }
    let magic = read_bits(&frame.pixels, 0, 8)?;
    if magic != u64::from(MAGIC) {
        return Err(CaptureError::BadBarcode);
    }
    let frame_no = read_bits(&frame.pixels, 8, 64)?;
    let timestamp_us = read_bits(&frame.pixels, 72, 64)?;
    Ok((frame_no, timestamp_us))
}

pub(crate) fn pixel_len(width: u32, height: u32) -> Result<usize, CaptureError> {
    let width = usize::try_from(width).map_err(|_| CaptureError::BadDimensions)?;
    let height = usize::try_from(height).map_err(|_| CaptureError::BadDimensions)?;
    width.checked_mul(height).ok_or(CaptureError::BadDimensions)
}

fn write_bits(pixels: &mut [u8], start: usize, value: u64, width: u32) {
    for offset in 0..width {
        let shift = width - 1 - offset;
        let on = ((value >> shift) & 1) == 1;
        pixels[start + offset as usize] = if on { 255 } else { 0 };
    }
}

fn read_bits(pixels: &[u8], start: usize, width: u32) -> Result<u64, CaptureError> {
    let end = start + width as usize;
    if pixels.len() < end {
        return Err(CaptureError::ShortFrame);
    }
    let mut value = 0_u64;
    for pixel in &pixels[start..end] {
        value = (value << 1) | u64::from(*pixel >= 128);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barcode_round_trips_and_narrow_frames_fail() {
        let capturer = SyntheticCapturer::new(BARCODE_WIDTH, 2);
        let frame = capturer.capture(7, 99).expect("frame");
        assert_eq!(read_barcode(&frame).expect("read"), (7, 99));
        let narrow = SyntheticCapturer::new(BARCODE_WIDTH - 1, 1);
        assert_eq!(narrow.capture(1, 1), Err(CaptureError::FrameTooNarrow));
    }
}

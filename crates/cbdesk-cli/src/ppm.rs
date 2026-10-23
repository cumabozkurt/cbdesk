// SPDX-License-Identifier: AGPL-3.0-or-later

//! A binary PPM so a headless viewer can keep the picture.

use std::path::Path;

use cbdesk_capture::Frame;

use crate::error::CliError;

/// Write grayscale `frame` as a color PPM with equal channels.
pub fn write_ppm(path: &Path, frame: &Frame) -> Result<(), CliError> {
    if frame.pixels.len() != frame.width as usize * frame.height as usize {
        return Err(CliError::new("decoded frame has the wrong length"));
    }
    let mut bytes = format!("P6\n{} {}\n255\n", frame.width, frame.height).into_bytes();
    bytes.reserve(frame.pixels.len().saturating_mul(3));
    for pixel in &frame.pixels {
        bytes.extend_from_slice(&[*pixel, *pixel, *pixel]);
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

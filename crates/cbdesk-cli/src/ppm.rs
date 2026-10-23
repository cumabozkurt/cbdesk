// SPDX-License-Identifier: AGPL-3.0-or-later

//! A binary PPM so a headless viewer can keep the picture.

use std::path::Path;

use cbdesk_capture::{yuv_to_rgb, Picture};

use crate::error::CliError;

/// Write `picture` as a color PPM. Neutral chroma stays gray, so a barcode
/// still has equal channels.
pub fn write_ppm(path: &Path, picture: &Picture) -> Result<(), CliError> {
    let width = picture.width as usize;
    let height = picture.height as usize;
    if picture.y.len() != width.saturating_mul(height) {
        return Err(CliError::new("decoded frame has the wrong length"));
    }
    let mut bytes = format!("P6\n{} {}\n255\n", picture.width, picture.height).into_bytes();
    bytes.reserve(picture.y.len().saturating_mul(3));
    for y in 0..picture.height {
        for x in 0..picture.width {
            let [red, green, blue] = sample(picture, x, y);
            bytes.extend_from_slice(&[red, green, blue]);
        }
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn sample(picture: &Picture, x: u32, y: u32) -> [u8; 3] {
    let width = picture.width.max(1) as usize;
    let luma_index = (y as usize)
        .saturating_mul(width)
        .saturating_add(x as usize);
    let luma = picture.y.get(luma_index).copied().unwrap_or(0);
    let cw = (picture.width / 2).max(1) as usize;
    let chroma_index = ((y / 2) as usize)
        .saturating_mul(cw)
        .saturating_add((x / 2) as usize);
    let chroma_u = picture.u.get(chroma_index).copied().unwrap_or(128);
    let chroma_v = picture.v.get(chroma_index).copied().unwrap_or(128);
    yuv_to_rgb(luma, chroma_u, chroma_v)
}

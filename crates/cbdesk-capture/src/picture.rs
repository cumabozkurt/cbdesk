// SPDX-License-Identifier: AGPL-3.0-or-later

//! A color picture in I420.
//!
//! Y is full resolution. U and V are half resolution on each axis. The live
//! path keeps this picture at the captured size, cropped to even dimensions,
//! and does not scale it down.

use crate::barcode::Frame;
use crate::error::CaptureError;

/// One I420 picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// Width in luma pixels. Even for a picture the encoder will accept.
    pub width: u32,
    /// Height in luma pixels.
    pub height: u32,
    /// Row-major luma, `width * height` bytes.
    pub y: Vec<u8>,
    /// Row-major U, `(width / 2) * (height / 2)` bytes when both axes are even.
    pub u: Vec<u8>,
    /// Row-major V, same length as [`Picture::u`].
    pub v: Vec<u8>,
}

impl Picture {
    /// Neutral-chroma picture from a grayscale frame.
    ///
    /// Width and height must be even and at least 2.
    pub fn from_luma(frame: &Frame) -> Result<Self, CaptureError> {
        if !frame.width.is_multiple_of(2) || !frame.height.is_multiple_of(2) {
            return Err(CaptureError::BadDimensions);
        }
        let luma = luma_len(frame.width, frame.height)?;
        if frame.pixels.len() != luma {
            return Err(CaptureError::ShortFrame);
        }
        let chroma = chroma_len(frame.width, frame.height)?;
        Ok(Self {
            width: frame.width,
            height: frame.height,
            y: frame.pixels.clone(),
            u: vec![128; chroma],
            v: vec![128; chroma],
        })
    }

    /// Grayscale view of the luma plane.
    pub fn to_luma(&self) -> Result<Frame, CaptureError> {
        let luma = luma_len(self.width, self.height)?;
        if self.y.len() != luma {
            return Err(CaptureError::ShortFrame);
        }
        Ok(Frame {
            width: self.width,
            height: self.height,
            pixels: self.y.clone(),
        })
    }
}

/// Build an I420 picture from tightly packed RGBA.
///
/// Odd sizes are kept. [`crop_even`] drops the last row or column before encode.
pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Picture, CaptureError> {
    let samples = luma_len(width, height)?;
    if rgba.len() != samples.saturating_mul(4) || samples == 0 {
        return Err(CaptureError::ShortFrame);
    }
    let mut y = vec![0_u8; samples];
    let mut pixels = Vec::with_capacity(samples);
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        let (luma, chroma_u, chroma_v) = yuv_of(pixel[0], pixel[1], pixel[2]);
        y[index] = luma;
        pixels.push((chroma_u, chroma_v));
    }
    let cw = width.div_ceil(2);
    let ch = height.div_ceil(2);
    let chroma = usize::try_from(cw)
        .ok()
        .and_then(|w| usize::try_from(ch).ok().and_then(|h| w.checked_mul(h)))
        .ok_or(CaptureError::BadDimensions)?;
    let mut u = vec![128_u8; chroma];
    let mut v = vec![128_u8; chroma];
    for cy in 0..ch {
        for cx in 0..cw {
            let mut sum_u = 0_i32;
            let mut sum_v = 0_i32;
            let mut count = 0_i32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let x = cx.saturating_mul(2).saturating_add(dx);
                    let py = cy.saturating_mul(2).saturating_add(dy);
                    if x < width && py < height {
                        let index = (py as usize).saturating_mul(width as usize) + x as usize;
                        if let Some((sample_u, sample_v)) = pixels.get(index) {
                            sum_u += i32::from(*sample_u);
                            sum_v += i32::from(*sample_v);
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                let slot = (cy as usize).saturating_mul(cw as usize) + cx as usize;
                if let Some(dest) = u.get_mut(slot) {
                    *dest = clamp_u8(sum_u / count);
                }
                if let Some(dest) = v.get_mut(slot) {
                    *dest = clamp_u8(sum_v / count);
                }
            }
        }
    }
    Ok(Picture {
        width,
        height,
        y,
        u,
        v,
    })
}

/// Drop the last row or column when a side is odd.
///
/// The result is not scaled. A picture that is already even is returned as-is.
pub fn crop_even(picture: Picture) -> Result<Picture, CaptureError> {
    if picture.width < 2 || picture.height < 2 {
        return Err(CaptureError::BadDimensions);
    }
    let width = picture.width & !1;
    let height = picture.height & !1;
    if width == picture.width && height == picture.height {
        let chroma = chroma_len(width, height)?;
        if picture.y.len() != luma_len(width, height)?
            || picture.u.len() != chroma
            || picture.v.len() != chroma
        {
            return Err(CaptureError::ShortFrame);
        }
        return Ok(picture);
    }
    let src_cw = picture.width.div_ceil(2) as usize;
    let dst_cw = (width / 2) as usize;
    let dst_ch = (height / 2) as usize;
    let mut y = Vec::with_capacity((width as usize).saturating_mul(height as usize));
    for row in 0..height as usize {
        let start = row.saturating_mul(picture.width as usize);
        let end = start.saturating_add(width as usize);
        let Some(slice) = picture.y.get(start..end) else {
            return Err(CaptureError::ShortFrame);
        };
        y.extend_from_slice(slice);
    }
    let u = crop_chroma(&picture.u, src_cw, dst_cw, dst_ch)?;
    let v = crop_chroma(&picture.v, src_cw, dst_cw, dst_ch)?;
    Ok(Picture {
        width,
        height,
        y,
        u,
        v,
    })
}

/// Expand one I420 sample to 8-bit RGB with the BT.601 limited-range matrix.
pub fn yuv_to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let c = i32::from(y) - 16;
    let d = i32::from(u) - 128;
    let e = i32::from(v) - 128;
    let r = (298 * c + 409 * e + 128) >> 8;
    let g = (298 * c - 100 * d - 208 * e + 128) >> 8;
    let b = (298 * c + 516 * d + 128) >> 8;
    [clamp_u8(r), clamp_u8(g), clamp_u8(b)]
}

fn crop_chroma(
    plane: &[u8],
    src_cw: usize,
    dst_cw: usize,
    dst_ch: usize,
) -> Result<Vec<u8>, CaptureError> {
    let mut out = Vec::with_capacity(dst_cw.saturating_mul(dst_ch));
    for row in 0..dst_ch {
        let start = row.saturating_mul(src_cw);
        let end = start.saturating_add(dst_cw);
        let Some(slice) = plane.get(start..end) else {
            return Err(CaptureError::ShortFrame);
        };
        out.extend_from_slice(slice);
    }
    Ok(out)
}

fn yuv_of(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = i32::from(r);
    let g = i32::from(g);
    let b = i32::from(b);
    let y = (77 * r + 150 * g + 29 * b) >> 8;
    let u = 128 + ((-43 * r - 85 * g + 128 * b) >> 8);
    let v = 128 + ((128 * r - 107 * g - 21 * b) >> 8);
    (clamp_u8(y), clamp_u8(u), clamp_u8(v))
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn luma_len(width: u32, height: u32) -> Result<usize, CaptureError> {
    usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .filter(|len| *len > 0)
        .ok_or(CaptureError::BadDimensions)
}

fn chroma_len(width: u32, height: u32) -> Result<usize, CaptureError> {
    let cw = width / 2;
    let ch = height / 2;
    usize::try_from(cw)
        .ok()
        .and_then(|w| usize::try_from(ch).ok().and_then(|h| w.checked_mul(h)))
        .filter(|len| *len > 0)
        .ok_or(CaptureError::BadDimensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_picture_crops_without_scaling() {
        let rgba = vec![10_u8; 1281 * 721 * 4];
        let picture = from_rgba(1281, 721, &rgba).expect("rgba");
        let even = crop_even(picture).expect("crop");
        assert_eq!((even.width, even.height), (1280, 720));
        assert_eq!(even.y.len(), 1280 * 720);
        assert_eq!(even.u.len(), 640 * 360);
    }

    #[test]
    fn red_green_and_blue_keep_a_dominant_channel() {
        for (rgba, channel) in [
            ([255_u8, 0, 0, 255], 0_usize),
            ([0, 255, 0, 255], 1),
            ([0, 0, 255, 255], 2),
        ] {
            let mut bytes = Vec::new();
            for _ in 0..4 {
                bytes.extend_from_slice(&rgba);
            }
            let picture = from_rgba(2, 2, &bytes).expect("rgba");
            let rgb = yuv_to_rgb(picture.y[0], picture.u[0], picture.v[0]);
            assert!(rgb[channel] > rgb[(channel + 1) % 3]);
            assert!(rgb[channel] > rgb[(channel + 2) % 3]);
        }
    }
}

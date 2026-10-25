// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shrink a frame so a software encoder can finish one picture.

use crate::barcode::{pixel_len, Frame};
use crate::error::CaptureError;

/// Scale `frame` so it fits inside `max_width` by `max_height`.
///
/// The result is even and at least 16 pixels on each side, which is the
/// smallest picture the AV1 encoder in this tree accepts. A frame that
/// already fits is returned unchanged.
pub fn fit_even(frame: &Frame, max_width: u32, max_height: u32) -> Result<Frame, CaptureError> {
    if frame.pixels.len() != pixel_len(frame.width, frame.height)? {
        return Err(CaptureError::ShortFrame);
    }
    let (width, height) = target_size(frame.width, frame.height, max_width, max_height)?;
    if width == frame.width && height == frame.height {
        return Ok(frame.clone());
    }
    scale_nearest(frame, width, height)
}

fn target_size(
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> Result<(u32, u32), CaptureError> {
    if width == 0 || height == 0 || max_width < 16 || max_height < 16 {
        return Err(CaptureError::BadDimensions);
    }
    let mut out_w = width;
    let mut out_h = height;
    if out_w > max_width || out_h > max_height {
        let scale = div_ceil(out_w, max_width).max(div_ceil(out_h, max_height));
        out_w /= scale.max(1);
        out_h /= scale.max(1);
    }
    Ok(((out_w.max(16)) & !1, (out_h.max(16)) & !1))
}

fn div_ceil(value: u32, divisor: u32) -> u32 {
    value.saturating_add(divisor.saturating_sub(1)) / divisor
}

fn scale_nearest(frame: &Frame, width: u32, height: u32) -> Result<Frame, CaptureError> {
    let mut pixels = vec![0_u8; pixel_len(width, height)?];
    for y in 0..height {
        let src_y = u64::from(y) * u64::from(frame.height) / u64::from(height);
        for x in 0..width {
            let src_x = u64::from(x) * u64::from(frame.width) / u64::from(width);
            let src = src_y * u64::from(frame.width) + src_x;
            let index = usize::try_from(src).map_err(|_| CaptureError::BadDimensions)?;
            let dest = usize::try_from(u64::from(y) * u64::from(width) + u64::from(x))
                .map_err(|_| CaptureError::BadDimensions)?;
            pixels[dest] = frame.pixels[index];
        }
    }
    Ok(Frame {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_frames_shrink_to_even_bounds() {
        let frame = Frame {
            width: 100,
            height: 50,
            pixels: vec![9_u8; 100 * 50],
        };
        let fitted = fit_even(&frame, 40, 40).expect("fit");
        assert!(fitted.width <= 40 && fitted.height <= 40);
        assert_eq!(fitted.width % 2, 0);
        assert_eq!(fitted.height % 2, 0);
        assert!(fitted.width >= 16 && fitted.height >= 16);
        assert_eq!(
            fitted.pixels.len(),
            fitted.width as usize * fitted.height as usize
        );
    }
}

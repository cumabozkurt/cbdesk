// SPDX-License-Identifier: AGPL-3.0-or-later

//! Timing for the software VP8 encoder.
//!
//! The numbers are wall time around [`Vp8Encoder::encode`]. They are not a
//! claim that a desktop session holds 30 frames per second. A 1080p run is
//! ignored by default because a pure-Rust 1920×1080 encode is slow and the
//! result belongs in `docs/dev/media-measurement.md` after someone runs it.

use std::time::Instant;

use cbdesk_capture::Picture;

use crate::error::CodecError;
use crate::vp8::Vp8Encoder;

/// One timed software VP8 sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vp8Timing {
    /// Luma width.
    pub width: u32,
    /// Luma height.
    pub height: u32,
    /// Pictures encoded.
    pub frames: u32,
    /// How many of those pictures were keyframes.
    pub keyframes: u32,
    /// Wall time for the whole sequence, in milliseconds.
    pub elapsed_ms: u128,
    /// Sum of the encoded packet lengths.
    pub bytes: usize,
}

/// Encode `frames` synthetic pictures and return the wall time.
///
/// The first picture is a keyframe. Later pictures of the same size are inter
/// frames. The pixels change with the frame index so the inter frame is not
/// a copy of the keyframe. `frames` must be at least 1. Dimensions must be
/// even and at least 16.
pub fn measure_vp8(
    width: u32,
    height: u32,
    frames: u32,
    qindex: u8,
) -> Result<Vp8Timing, CodecError> {
    if frames == 0 {
        return Err(CodecError::BadDimensions);
    }
    let mut encoder = Vp8Encoder::new();
    let started = Instant::now();
    let mut keyframes = 0_u32;
    let mut bytes = 0_usize;
    for index in 0..frames {
        let picture = synthetic(width, height, index as u8)?;
        let (packet, keyframe) = encoder.encode(&picture, qindex)?;
        if keyframe {
            keyframes = keyframes.saturating_add(1);
        }
        bytes = bytes.saturating_add(packet.len());
    }
    Ok(Vp8Timing {
        width,
        height,
        frames,
        keyframes,
        elapsed_ms: started.elapsed().as_millis(),
        bytes,
    })
}

fn synthetic(width: u32, height: u32, phase: u8) -> Result<Picture, CodecError> {
    if width < 16 || height < 16 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    let luma = (width as usize).saturating_mul(height as usize);
    let chroma = (width as usize / 2).saturating_mul(height as usize / 2);
    let mut y = vec![0_u8; luma];
    for (index, pixel) in y.iter_mut().enumerate() {
        *pixel = phase.wrapping_mul(16).wrapping_add((index % 64) as u8);
    }
    let block = (width as usize / 8).saturating_add((phase as usize) * 16);
    if block + 16 <= width as usize {
        for row in 0..16.min(height as usize) {
            let start = row.saturating_mul(width as usize).saturating_add(block);
            for pixel in &mut y[start..start + 16] {
                *pixel = 255;
            }
        }
    }
    Ok(Picture {
        width,
        height,
        y,
        u: vec![128; chroma],
        v: vec![128; chroma],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_sequence_records_one_keyframe_and_a_positive_duration() {
        let timing = measure_vp8(32, 16, 2, 48).expect("timing");
        assert_eq!(timing.frames, 2);
        assert_eq!(timing.keyframes, 1);
        assert!(timing.bytes > 0);
        assert!(timing.elapsed_ms < 60_000);
    }

    #[test]
    #[ignore = "local strip timing; not a 30 fps claim and not run by cargo test"]
    fn time_parallel_strips_on_a_64_pixel_picture() {
        let picture = synthetic(64, 64, 1).expect("picture");
        let set = crate::encode_strips(&picture, 32, 4, true).expect("strips");
        assert_eq!(set.packets.len(), 4);
        assert!(set.wall_ms < 60_000);
    }

    #[test]
    #[ignore = "refresh docs/dev/media-measurement.md; pure-Rust 1080p is slow"]
    fn vp8_1080p_two_frames() {
        let handle = std::thread::Builder::new()
            .name("vp8-1080p".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| measure_vp8(1920, 1080, 2, 32).expect("1080p"))
            .expect("spawn");
        let timing = handle.join().expect("join");
        eprintln!(
            "vp8 1080p frames={} keyframes={} elapsed_ms={} bytes={}",
            timing.frames, timing.keyframes, timing.elapsed_ms, timing.bytes
        );
        assert_eq!(timing.width, 1920);
        assert_eq!(timing.height, 1080);
        assert_eq!(timing.frames, 2);
        assert_eq!(timing.keyframes, 1);
        assert!(timing.bytes > 0);
    }
}

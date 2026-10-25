// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Software video for one picture.
//!
//! AV1 uses rav1e with assembler and git tooling disabled, and rav1d-safe
//! under AGPL-3.0-only. VP8 uses oxideav-vp8, which is pure Rust and does
//! not need a system libvpx. A stream is a keyframe followed by inter frames.
//! AV1 in this tree is still one still picture.

mod bench;
mod cmp;
mod damage;
mod error;
mod hardware;
mod profile;
mod rate;
mod vp8;

use cbdesk_capture::Frame as Picture;
use rav1d_safe::src::managed::{Decoder, Planes};
use rav1e::prelude::*;

pub use bench::{measure_vp8, Vp8Timing};
pub use damage::{
    composite_vp8, measure_damage_path, DamageEncoder, DamagePacket, DamageTiming, PlacedFrame,
};
pub use error::CodecError;
pub use hardware::{
    encode_i420_with_ffmpeg, ffmpeg_encoder_listed, open_hardware, probe_hardware, require_chroma,
    ChromaFormat, EncodedPacket, FrameEncoder, HardwareKind, HardwareProbe, HardwareSession,
    QualityPreset, HARDWARE_KINDS,
};
pub use profile::{
    composite_strips, encode_strips, measure_1080p_paths, scale_box, scale_profile, FrameProfile,
    PathTiming, StripPacket, StripSet,
};
pub use rate::{BitrateControl, Pace};
pub use vp8::{decode_vp8_luma, encode_vp8, vp8_is_inter, Vp8Encoder, Vp8StreamDecoder};

/// How hard the encoder should try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeQuality {
    /// Quantizer 0. Barcode tests use this so luma stays 0 or 255.
    Lossless,
    /// A fast still for a live desktop preview.
    Fast,
}

/// Encode `frame` as one AV1 still picture.
///
/// Width and height must be even and at least 16. U and V are neutral gray.
pub fn encode_luma(frame: &Picture, quality: EncodeQuality) -> Result<Vec<u8>, CodecError> {
    let width = usize::try_from(frame.width).map_err(|_| CodecError::BadDimensions)?;
    let height = usize::try_from(frame.height).map_err(|_| CodecError::BadDimensions)?;
    if width < 16 || height < 16 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    if frame.pixels.len() != width.saturating_mul(height) {
        return Err(CodecError::BadDimensions);
    }
    let mut encoded = encoder(width, height, quality)?;
    let mut picture = encoded.new_frame();
    write_planes(&mut picture, &frame.pixels, width)?;
    encoded
        .send_frame(picture)
        .map_err(|err| CodecError::Encode(err.to_string()))?;
    encoded.flush();
    take_packet(&mut encoded)
}

/// Decode one AV1 payload to grayscale.
pub fn decode_luma(bytes: &[u8]) -> Result<Picture, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::NoFrame);
    }
    let mut decoder = Decoder::new().map_err(|err| CodecError::Decode(err.to_string()))?;
    let picture = match decoder
        .decode(bytes)
        .map_err(|err| CodecError::Decode(err.to_string()))?
    {
        Some(picture) => picture,
        None => decoder
            .flush()
            .map_err(|err| CodecError::Decode(err.to_string()))?
            .into_iter()
            .next()
            .ok_or(CodecError::NoFrame)?,
    };
    copy_luma(&picture)
}

fn encoder(width: usize, height: usize, quality: EncodeQuality) -> Result<Context<u8>, CodecError> {
    let speed = match quality {
        // Preset 10 forces 32x32 blocks, which smear a 16-pixel barcode.
        EncodeQuality::Lossless => 5,
        EncodeQuality::Fast => 10,
    };
    let mut config = EncoderConfig::with_speed_preset(speed);
    config.width = width;
    config.height = height;
    config.still_picture = true;
    config.low_latency = true;
    config.pixel_range = PixelRange::Full;
    config.bitrate = 0;
    config.min_quantizer = 0;
    if matches!(quality, EncodeQuality::Lossless) {
        config.speed_settings.cdef = false;
        config.speed_settings.lrf = false;
    }
    config.quantizer = match quality {
        EncodeQuality::Lossless => 0,
        EncodeQuality::Fast => 120,
    };
    Config::new()
        .with_threads(1)
        .with_encoder_config(config)
        .new_context()
        .map_err(|err| CodecError::Encode(err.to_string()))
}

fn write_planes(picture: &mut Frame<u8>, luma: &[u8], width: usize) -> Result<(), CodecError> {
    copy_plane(&mut picture.planes[0], luma, width)?;
    for plane in picture.planes.iter_mut().skip(1) {
        fill_plane(plane, 128)?;
    }
    Ok(())
}

fn copy_plane(plane: &mut Plane<u8>, samples: &[u8], width: usize) -> Result<(), CodecError> {
    for (y, row) in plane.rows_iter_mut().enumerate() {
        let start = y.saturating_mul(width);
        let end = start.saturating_add(width.min(row.len()));
        if end > samples.len() || end - start != row.len().min(width) {
            return Err(CodecError::BadDimensions);
        }
        let count = width.min(row.len());
        row[..count].copy_from_slice(&samples[start..start + count]);
    }
    Ok(())
}

fn fill_plane(plane: &mut Plane<u8>, value: u8) -> Result<(), CodecError> {
    for row in plane.rows_iter_mut() {
        row.fill(value);
    }
    Ok(())
}

fn take_packet(encoded: &mut Context<u8>) -> Result<Vec<u8>, CodecError> {
    let mut payload = Vec::new();
    for _ in 0..8 {
        match encoded.receive_packet() {
            Ok(packet) => payload.extend_from_slice(&packet.data),
            Err(EncoderStatus::Encoded) => continue,
            Err(EncoderStatus::LimitReached | EncoderStatus::NeedMoreData) => break,
            Err(err) => return Err(CodecError::Encode(err.to_string())),
        }
    }
    if payload.is_empty() {
        return Err(CodecError::NoFrame);
    }
    Ok(payload)
}

fn copy_luma(picture: &rav1d_safe::src::managed::Frame) -> Result<Picture, CodecError> {
    let Planes::Depth8(planes) = picture.planes() else {
        return Err(CodecError::Decode("frame is not 8-bit".to_string()));
    };
    let y = planes.y();
    let width = u32::try_from(y.width()).map_err(|_| CodecError::BadDimensions)?;
    let height = u32::try_from(y.height()).map_err(|_| CodecError::BadDimensions)?;
    let mut pixels = Vec::with_capacity(y.width().saturating_mul(y.height()));
    for row in 0..y.height() {
        pixels.extend_from_slice(y.row(row));
    }
    if pixels.len() != y.width().saturating_mul(y.height()) {
        return Err(CodecError::Decode("luma plane was short".to_string()));
    }
    Ok(Picture {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use cbdesk_capture::{read_barcode, SyntheticCapturer, BARCODE_WIDTH};

    use super::*;

    #[test]
    fn lossless_barcode_survives_av1() {
        let original = SyntheticCapturer::new(BARCODE_WIDTH, 16)
            .capture(7, 99)
            .expect("barcode");
        let packet = encode_luma(&original, EncodeQuality::Lossless).expect("encode");
        let decoded = decode_luma(&packet).expect("decode");
        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
        assert_eq!(read_barcode(&decoded).expect("read"), (7, 99));
        assert!(decoded
            .pixels
            .iter()
            .all(|pixel| *pixel == 0 || *pixel == 255));
    }
}

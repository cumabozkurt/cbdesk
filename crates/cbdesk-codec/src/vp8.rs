// SPDX-License-Identifier: AGPL-3.0-or-later

//! VP8 keyframes through oxideav-vp8.
//!
//! The crate is pure Rust. Its registry feature, which pulls another
//! framework, stays off. Each call emits one keyframe. The decoder entry
//! used here is stateless and accepts those keyframes. A stream is a
//! sequence of them, not an inter-frame chain.

use cbdesk_capture::Frame as Picture;
use oxideav_vp8::{decode_vp8, encode_keyframe, I420Frame, KeyframeParams, TrellisStrength};

use crate::{CodecError, EncodeQuality};

/// Encode `frame` as one VP8 keyframe.
///
/// Width and height must be even and at least 16. U and V are neutral gray.
pub fn encode_vp8(frame: &Picture, quality: EncodeQuality) -> Result<Vec<u8>, CodecError> {
    let (width, height) = dimensions(frame)?;
    let chroma = neutral_chroma(width, height)?;
    let picture = I420Frame::packed(width, height, &frame.pixels, &chroma, &chroma);
    encode_keyframe(&picture, &params(quality)).map_err(|err| CodecError::Encode(err.to_string()))
}

/// Decode one VP8 keyframe to grayscale.
pub fn decode_vp8_luma(bytes: &[u8]) -> Result<Picture, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::NoFrame);
    }
    let picture = decode_vp8(bytes).map_err(|err| CodecError::Decode(err.to_string()))?;
    let width = picture.width;
    let height = picture.height;
    if picture.y.len() != (width as usize).saturating_mul(height as usize) {
        return Err(CodecError::Decode("luma plane was short".to_string()));
    }
    Ok(Picture {
        width,
        height,
        pixels: picture.y,
    })
}

fn params(quality: EncodeQuality) -> KeyframeParams {
    KeyframeParams {
        y_ac_qi: match quality {
            EncodeQuality::Lossless => 0,
            EncodeQuality::Fast => 48,
        },
        loop_filter_level: 0,
        sharpness_level: 0,
        nbr_of_dct_partitions: 1,
        filter_type: false,
        trellis_strength: TrellisStrength::OFF,
    }
}

fn dimensions(frame: &Picture) -> Result<(u32, u32), CodecError> {
    if frame.width < 16
        || frame.height < 16
        || !frame.width.is_multiple_of(2)
        || !frame.height.is_multiple_of(2)
    {
        return Err(CodecError::BadDimensions);
    }
    let len = (frame.width as usize).saturating_mul(frame.height as usize);
    if frame.pixels.len() != len {
        return Err(CodecError::BadDimensions);
    }
    Ok((frame.width, frame.height))
}

fn neutral_chroma(width: u32, height: u32) -> Result<Vec<u8>, CodecError> {
    let chroma_w = (width / 2) as usize;
    let chroma_h = (height / 2) as usize;
    let len = chroma_w
        .checked_mul(chroma_h)
        .ok_or(CodecError::BadDimensions)?;
    Ok(vec![128_u8; len])
}

#[cfg(test)]
mod tests {
    use cbdesk_capture::{read_barcode, SyntheticCapturer, BARCODE_WIDTH};

    use super::*;

    #[test]
    fn vp8_barcode_stream_stays_readable() {
        // T-MED-004 for this build: a short VP8 keyframe stream whose barcode
        // still reads. It is not a 60-second soak and not an inter frame.
        for (frame_no, timestamp) in [(1_u64, 1_000_u64), (7, 99), (42, 1_000)] {
            let original = SyntheticCapturer::new(BARCODE_WIDTH, 16)
                .capture(frame_no, timestamp)
                .expect("barcode");
            let packet = encode_vp8(&original, EncodeQuality::Lossless).expect("encode");
            let decoded = decode_vp8_luma(&packet).expect("decode");
            assert_eq!(decoded.width, original.width);
            assert_eq!(decoded.height, original.height);
            assert_eq!(read_barcode(&decoded).expect("read"), (frame_no, timestamp));
        }
    }
}

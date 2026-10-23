// SPDX-License-Identifier: AGPL-3.0-or-later

//! VP8 through oxideav-vp8.
//!
//! The crate is pure Rust. Its registry feature, which pulls another
//! framework, stays off. [`encode_vp8`] still emits one keyframe.
//! [`Vp8Encoder`] emits a keyframe and then zero-motion inter frames.
//! [`Vp8StreamDecoder`] keeps the reference state those inter frames need.
//! The stateless [`decode_vp8_luma`] entry accepts keyframes only.

use cbdesk_capture::{Frame, Picture};
use oxideav_vp8::frame::KeyframePlanes;
use oxideav_vp8::state::Vp8DecoderState;
use oxideav_vp8::{
    decode_vp8, encode_keyframe, encode_keyframe_with_reconstruction, encode_p_frame_zero_mv,
    I420Frame, KeyframeParams, TrellisStrength,
};

use crate::{CodecError, EncodeQuality};

/// Encode `frame` as one VP8 keyframe.
///
/// Width and height must be even and at least 16. U and V are neutral gray.
pub fn encode_vp8(frame: &Frame, quality: EncodeQuality) -> Result<Vec<u8>, CodecError> {
    let (width, height) = dimensions(frame)?;
    let chroma = neutral_chroma(width, height)?;
    let picture = I420Frame::packed(width, height, &frame.pixels, &chroma, &chroma);
    encode_keyframe(&picture, &params(quality)).map_err(|err| CodecError::Encode(err.to_string()))
}

/// Decode one VP8 keyframe to grayscale.
pub fn decode_vp8_luma(bytes: &[u8]) -> Result<Frame, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::NoFrame);
    }
    let picture = decode_vp8(bytes).map_err(|err| CodecError::Decode(err.to_string()))?;
    let width = picture.width;
    let height = picture.height;
    if picture.y.len() != (width as usize).saturating_mul(height as usize) {
        return Err(CodecError::Decode("luma plane was short".to_string()));
    }
    Ok(Frame {
        width,
        height,
        pixels: picture.y,
    })
}

/// Encoder state for one VP8 stream.
///
/// The first picture, and any later picture whose size changed, is a keyframe.
/// Later pictures of the same size are zero-motion inter frames. The frame tag's
/// low bit is 1 on those packets.
#[derive(Debug)]
pub struct Vp8Encoder {
    reference: Option<KeyframePlanes>,
    size: Option<(u32, u32)>,
}

impl Vp8Encoder {
    /// An encoder that has not sent a keyframe yet.
    pub fn new() -> Self {
        Self {
            reference: None,
            size: None,
        }
    }

    /// Encode `picture` at `qindex` (`y_ac_qi`, clamped to 0..=127).
    ///
    /// The boolean is true when the packet is a keyframe.
    pub fn encode(&mut self, picture: &Picture, qindex: u8) -> Result<(Vec<u8>, bool), CodecError> {
        let packed = pack(picture)?;
        let params = stream_params(qindex);
        let keyframe =
            self.reference.is_none() || self.size != Some((picture.width, picture.height));
        let (bytes, planes) = if keyframe {
            encode_keyframe_with_reconstruction(&packed, &params)
                .map_err(|err| CodecError::Encode(err.to_string()))?
        } else {
            let reference = self
                .reference
                .as_ref()
                .ok_or_else(|| CodecError::Encode("inter frame has no reference".to_string()))?;
            encode_p_frame_zero_mv(&packed, reference, &params)
                .map_err(|err| CodecError::Encode(err.to_string()))?
        };
        self.reference = Some(planes);
        self.size = Some((picture.width, picture.height));
        Ok((bytes, keyframe))
    }
}

impl Default for Vp8Encoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decoder state for one VP8 stream.
///
/// The first packet must be a keyframe. An inter frame updates that reference.
#[derive(Debug)]
pub struct Vp8StreamDecoder {
    state: Vp8DecoderState,
}

impl Vp8StreamDecoder {
    /// A decoder that has not seen a keyframe yet.
    pub fn new() -> Self {
        Self {
            state: Vp8DecoderState::new(),
        }
    }

    /// Decode one packet to I420.
    pub fn decode(&mut self, bytes: &[u8]) -> Result<Picture, CodecError> {
        if bytes.is_empty() {
            return Err(CodecError::NoFrame);
        }
        let picture = self
            .state
            .decode_frame(bytes)
            .map_err(|err| CodecError::Decode(err.to_string()))?;
        let chroma = chroma_len(picture.width, picture.height)?;
        if picture.y.len() != luma_len(picture.width, picture.height)?
            || picture.u.len() != chroma
            || picture.v.len() != chroma
        {
            return Err(CodecError::Decode("decoded planes were short".to_string()));
        }
        Ok(Picture {
            width: picture.width,
            height: picture.height,
            y: picture.y,
            u: picture.u,
            v: picture.v,
        })
    }
}

impl Default for Vp8StreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// True when the VP8 frame tag says this packet is an inter frame.
pub fn vp8_is_inter(bytes: &[u8]) -> bool {
    bytes.first().is_some_and(|tag| tag & 1 == 1)
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

fn stream_params(qindex: u8) -> KeyframeParams {
    KeyframeParams {
        y_ac_qi: qindex.min(127),
        loop_filter_level: 0,
        sharpness_level: 0,
        nbr_of_dct_partitions: 1,
        filter_type: false,
        trellis_strength: TrellisStrength::OFF,
    }
}

fn pack(picture: &Picture) -> Result<I420Frame<'_>, CodecError> {
    if picture.width < 16
        || picture.height < 16
        || !picture.width.is_multiple_of(2)
        || !picture.height.is_multiple_of(2)
    {
        return Err(CodecError::BadDimensions);
    }
    let luma = luma_len(picture.width, picture.height)?;
    let chroma = chroma_len(picture.width, picture.height)?;
    if picture.y.len() != luma || picture.u.len() != chroma || picture.v.len() != chroma {
        return Err(CodecError::BadDimensions);
    }
    Ok(I420Frame::packed(
        picture.width,
        picture.height,
        &picture.y,
        &picture.u,
        &picture.v,
    ))
}

fn luma_len(width: u32, height: u32) -> Result<usize, CodecError> {
    usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or(CodecError::BadDimensions)
}

fn chroma_len(width: u32, height: u32) -> Result<usize, CodecError> {
    let cw = (width / 2) as usize;
    let ch = (height / 2) as usize;
    cw.checked_mul(ch).ok_or(CodecError::BadDimensions)
}

fn dimensions(frame: &Frame) -> Result<(u32, u32), CodecError> {
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
        // still reads. It is not a 60-second soak. Inter frames are the next test.
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

    #[test]
    fn an_inter_frame_keeps_the_barcode_and_the_frame_tag() {
        let first = SyntheticCapturer::new(BARCODE_WIDTH, 16)
            .capture(1, 1_000)
            .expect("first");
        let second = SyntheticCapturer::new(BARCODE_WIDTH, 16)
            .capture(7, 99)
            .expect("second");
        let mut encoder = Vp8Encoder::new();
        let mut decoder = Vp8StreamDecoder::new();
        let (key, keyframe) = encoder
            .encode(&Picture::from_luma(&first).expect("luma"), 0)
            .expect("key");
        assert!(keyframe);
        assert!(!vp8_is_inter(&key));
        let decoded = decoder.decode(&key).expect("decode key");
        assert_eq!(
            read_barcode(&decoded.to_luma().expect("luma")).expect("read"),
            (1, 1_000)
        );
        let (inter, keyframe) = encoder
            .encode(&Picture::from_luma(&second).expect("luma"), 0)
            .expect("inter");
        assert!(!keyframe);
        assert!(vp8_is_inter(&inter));
        let decoded = decoder.decode(&inter).expect("decode inter");
        assert_eq!(
            read_barcode(&decoded.to_luma().expect("luma")).expect("read"),
            (7, 99)
        );
    }

    #[test]
    fn color_survives_a_keyframe_and_an_inter_frame() {
        let red = solid(255, 0, 0);
        let green = solid(0, 255, 0);
        let mut encoder = Vp8Encoder::new();
        let mut decoder = Vp8StreamDecoder::new();
        let (key, _) = encoder.encode(&red, 0).expect("key");
        let decoded = decoder.decode(&key).expect("decode");
        assert!(dominant(&decoded, 0));
        let (inter, keyframe) = encoder.encode(&green, 0).expect("inter");
        assert!(!keyframe);
        assert!(vp8_is_inter(&inter));
        let decoded = decoder.decode(&inter).expect("decode inter");
        assert!(dominant(&decoded, 1));
    }

    fn solid(r: u8, g: u8, b: u8) -> Picture {
        let mut rgba = Vec::new();
        for _ in 0..(16 * 16) {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
        cbdesk_capture::from_rgba(16, 16, &rgba).expect("rgba")
    }

    fn dominant(picture: &Picture, channel: usize) -> bool {
        let rgb = cbdesk_capture::yuv_to_rgb(picture.y[0], picture.u[0], picture.v[0]);
        rgb[channel] > rgb[(channel + 1) % 3] && rgb[channel] > rgb[(channel + 2) % 3]
    }
}

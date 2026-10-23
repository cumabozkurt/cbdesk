// SPDX-License-Identifier: AGPL-3.0-or-later

//! One motion-compensated VP8 pair the remote view can show.
//!
//! The picture is 48×16. It is the same shift the codec test locks in.
//! It is not a measured 1080p capture.

use cbdesk_capture::Picture;
use cbdesk_codec::{vp8_is_inter, Vp8Encoder, Vp8StreamDecoder};

use crate::error::DeskError;

/// One packet the host can send without capturing the display.
pub struct PreviewPacket {
    /// VP8 bytes.
    pub bytes: Vec<u8>,
    /// True when a decoder can start on this packet.
    pub keyframe: bool,
    /// Picture those bytes represent.
    pub picture: Picture,
}

/// Keyframe, inter frame, and the decoded inter picture.
pub struct MotionPair {
    /// First packet. A decoder can start here.
    pub key: Vec<u8>,
    /// Second packet. The frame tag says this one is an inter frame.
    pub inter: Vec<u8>,
    /// Decoded inter picture, I420.
    pub picture: Picture,
}

/// The motion pair as two packets the host loop can send in order.
pub fn motion_preview() -> Result<Vec<PreviewPacket>, DeskError> {
    let pair = motion_pair()?;
    Ok(vec![
        PreviewPacket {
            bytes: pair.key,
            keyframe: true,
            picture: band(0),
        },
        PreviewPacket {
            bytes: pair.inter,
            keyframe: false,
            picture: pair.picture,
        },
    ])
}

/// Encode a white block, then the same block 16 pixels to the right.
pub fn motion_pair() -> Result<MotionPair, DeskError> {
    let mut encoder = Vp8Encoder::new();
    let (key, keyframe) = encoder
        .encode(&band(0), 32)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    if !keyframe || vp8_is_inter(&key) {
        return Err(DeskError::Message("first frame was not a keyframe".into()));
    }
    let (inter, keyframe) = encoder
        .encode(&band(16), 96)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    if keyframe || !vp8_is_inter(&inter) {
        return Err(DeskError::Message(
            "second frame was not an inter frame".into(),
        ));
    }
    let mut decoder = Vp8StreamDecoder::new();
    decoder
        .decode(&key)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    let picture = decoder
        .decode(&inter)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    Ok(MotionPair {
        key,
        inter,
        picture,
    })
}

fn band(origin: u32) -> Picture {
    let width = 48_u32;
    let height = 16_u32;
    let mut y = vec![0_u8; (width * height) as usize];
    for row in 0..height {
        for col in origin..origin + 16 {
            y[(row * width + col) as usize] = 255;
        }
    }
    let chroma = ((width / 2) * (height / 2)) as usize;
    Picture {
        width,
        height,
        y,
        u: vec![128; chroma],
        v: vec![128; chroma],
    }
}

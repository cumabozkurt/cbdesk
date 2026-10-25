// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardware encoders are named and unavailable.
//!
//! NVENC, AMF, Quick Sync, VA-API, VideoToolbox, MediaCodec, and D3D11VA are
//! not linked. Opening one returns [`CodecError::Unavailable`]. The software
//! VP8 encoder remains the path that produces bytes. VP8 in this tree is
//! 4:2:0. A 4:4:4 request is refused.

use cbdesk_capture::Picture;

use crate::error::CodecError;
use crate::vp8::Vp8Encoder;

/// A named hardware backend this build does not include.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareKind {
    /// NVIDIA NVENC.
    Nvenc,
    /// AMD AMF.
    Amf,
    /// Intel Quick Sync.
    QuickSync,
    /// Linux VA-API.
    Vaapi,
    /// Apple VideoToolbox.
    VideoToolbox,
    /// Android MediaCodec.
    MediaCodec,
    /// Windows D3D11 video acceleration.
    D3d11va,
}

impl HardwareKind {
    /// Stable name for logs and the diagnostic report.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nvenc => "nvenc",
            Self::Amf => "amf",
            Self::QuickSync => "qsv",
            Self::Vaapi => "vaapi",
            Self::VideoToolbox => "videotoolbox",
            Self::MediaCodec => "mediacodec",
            Self::D3d11va => "d3d11va",
        }
    }
}

/// Every backend this module knows about.
pub const HARDWARE_KINDS: [HardwareKind; 7] = [
    HardwareKind::Nvenc,
    HardwareKind::Amf,
    HardwareKind::QuickSync,
    HardwareKind::Vaapi,
    HardwareKind::VideoToolbox,
    HardwareKind::MediaCodec,
    HardwareKind::D3d11va,
];

/// Chroma subsampling a picture can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromaFormat {
    /// U and V at half resolution on each axis. Software VP8 uses this.
    Yuv420,
    /// Full-resolution chroma. Not produced by the software VP8 path.
    Yuv444,
}

/// What a quality preset asks the software quantizer to do.
///
/// These are index choices, not a measured visual score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityPreset {
    /// A finer quantizer.
    BestPicture,
    /// The same starting index as the live pacer.
    Balanced,
    /// A coarser quantizer.
    LowestLatency,
    /// Caller-chosen index, clamped to 0..=127.
    Custom(u8),
}

impl QualityPreset {
    /// VP8 `y_ac_qi` for this preset.
    pub const fn qindex(self) -> u8 {
        match self {
            Self::BestPicture => 16,
            Self::Balanced => 32,
            Self::LowestLatency => 48,
            Self::Custom(index) => {
                if index > 127 {
                    127
                } else {
                    index
                }
            }
        }
    }
}

/// One encoded packet and whether it was a keyframe.
pub struct EncodedPacket {
    /// Codec bytes.
    pub bytes: Vec<u8>,
    /// True when the packet is a keyframe.
    pub keyframe: bool,
}

/// Something that can encode one picture.
pub trait FrameEncoder {
    /// Encode `picture` at `qindex`.
    fn encode_frame(&mut self, picture: &Picture, qindex: u8) -> Result<EncodedPacket, CodecError>;
}

impl FrameEncoder for Vp8Encoder {
    fn encode_frame(&mut self, picture: &Picture, qindex: u8) -> Result<EncodedPacket, CodecError> {
        let (bytes, keyframe) = self.encode(picture, qindex)?;
        Ok(EncodedPacket { bytes, keyframe })
    }
}

/// Probe result for one hardware backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareProbe {
    /// Which backend was asked about.
    pub kind: HardwareKind,
    /// Always unavailable in this build.
    pub available: bool,
    /// Why it is unavailable.
    pub reason: &'static str,
}

/// Report every hardware backend as unavailable.
pub fn probe_hardware() -> [HardwareProbe; 7] {
    HARDWARE_KINDS.map(|kind| HardwareProbe {
        kind,
        available: false,
        reason: "not linked in this build",
    })
}

/// Refuse to open a hardware encoder.
pub fn open_hardware(kind: HardwareKind) -> Result<(), CodecError> {
    Err(CodecError::Unavailable(kind.name()))
}

/// Software VP8 chroma, or an error when `format` is 4:4:4.
pub fn require_chroma(format: ChromaFormat) -> Result<ChromaFormat, CodecError> {
    match format {
        ChromaFormat::Yuv420 => Ok(ChromaFormat::Yuv420),
        ChromaFormat::Yuv444 => Err(CodecError::Unavailable(
            "software VP8 in this tree is 4:2:0",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_capture::Picture;

    #[test]
    fn every_hardware_backend_is_unavailable_and_444_is_refused() {
        for probe in probe_hardware() {
            assert!(!probe.available);
            assert!(open_hardware(probe.kind).is_err());
        }
        assert_eq!(
            require_chroma(ChromaFormat::Yuv420).expect("420"),
            ChromaFormat::Yuv420
        );
        assert!(require_chroma(ChromaFormat::Yuv444).is_err());
        assert_eq!(QualityPreset::Balanced.qindex(), 32);
        assert_eq!(QualityPreset::Custom(200).qindex(), 127);
    }

    #[test]
    fn the_software_encoder_implements_the_same_trait() {
        let picture = Picture {
            width: 16,
            height: 16,
            y: vec![20; 256],
            u: vec![128; 64],
            v: vec![128; 64],
        };
        let mut encoder = Vp8Encoder::new();
        let packet = encoder
            .encode_frame(&picture, QualityPreset::LowestLatency.qindex())
            .expect("encode");
        assert!(packet.keyframe);
        assert!(!packet.bytes.is_empty());
    }
}

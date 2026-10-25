// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardware encoders behind cargo features.
//!
//! `hw-vaapi`, `hw-mediafoundation`, `hw-videotoolbox`, and `hw-mediacodec`
//! are off unless a build asks for them. A flag that is on shells out to
//! `ffmpeg` and asks for that platform's VP8 encoder name. There is no
//! in-process VA-API or VideoToolbox context in this crate. A missing
//! binary, a missing encoder name, or a missing device is
//! [`CodecError::Unavailable`]. Software VP8 stays 4:2:0. A 4:4:4 request
//! is refused. The quantizer index is not forwarded to ffmpeg.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cbdesk_capture::Picture;

use crate::error::CodecError;
use crate::vp8::Vp8Encoder;

/// A named hardware backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareKind {
    /// NVIDIA NVENC. No feature flag and no encoder in this tree.
    Nvenc,
    /// AMD AMF. No feature flag and no encoder in this tree.
    Amf,
    /// Intel Quick Sync. No feature flag and no encoder in this tree.
    QuickSync,
    /// Linux VA-API, feature `hw-vaapi`, encoder `vp8_vaapi`.
    Vaapi,
    /// Apple VideoToolbox, feature `hw-videotoolbox`, encoder `vp8_videotoolbox`.
    VideoToolbox,
    /// Android MediaCodec, feature `hw-mediacodec`, encoder `vp8_mediacodec`.
    MediaCodec,
    /// Windows Media Foundation, feature `hw-mediafoundation`, encoder `vp8_mf`.
    MediaFoundation,
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
            Self::MediaFoundation => "mediafoundation",
        }
    }

    const fn ffmpeg_encoder(self) -> Option<&'static str> {
        match self {
            Self::Vaapi => Some("vp8_vaapi"),
            Self::VideoToolbox => Some("vp8_videotoolbox"),
            Self::MediaCodec => Some("vp8_mediacodec"),
            Self::MediaFoundation => Some("vp8_mf"),
            Self::Nvenc | Self::Amf | Self::QuickSync => None,
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
    HardwareKind::MediaFoundation,
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
    /// True when this build's feature, the ffmpeg encoder, and the device all answered.
    pub available: bool,
    /// Why it is available or not.
    pub reason: &'static str,
}

/// An ffmpeg process that can encode one I420 picture.
#[derive(Debug)]
pub struct HardwareSession {
    kind: HardwareKind,
    ffmpeg: PathBuf,
    encoder: &'static str,
    device: Option<PathBuf>,
}

impl HardwareSession {
    /// Which backend opened.
    pub const fn kind(&self) -> HardwareKind {
        self.kind
    }

    /// Encode one picture. `qindex` is accepted and not passed to ffmpeg.
    pub fn encode(&self, picture: &Picture, qindex: u8) -> Result<Vec<u8>, CodecError> {
        let _ = qindex;
        let mut extra = Vec::new();
        if self.kind == HardwareKind::Vaapi {
            let device = self
                .device
                .as_ref()
                .ok_or(CodecError::Unavailable("VA-API render node is missing"))?;
            extra.push("-vaapi_device".to_string());
            extra.push(device.to_string_lossy().into_owned());
            extra.push("-vf".to_string());
            extra.push("format=nv12,hwupload".to_string());
        }
        encode_i420_with_ffmpeg(&self.ffmpeg, self.encoder, picture, &extra)
    }
}

impl FrameEncoder for HardwareSession {
    fn encode_frame(&mut self, picture: &Picture, qindex: u8) -> Result<EncodedPacket, CodecError> {
        let bytes = self.encode(picture, qindex)?;
        Ok(EncodedPacket {
            bytes,
            keyframe: true,
        })
    }
}

/// Report every hardware backend.
pub fn probe_hardware() -> [HardwareProbe; 7] {
    HARDWARE_KINDS.map(|kind| {
        let (available, reason) = probe_kind(kind);
        HardwareProbe {
            kind,
            available,
            reason,
        }
    })
}

/// Open one hardware encoder, or return why this build cannot.
pub fn open_hardware(kind: HardwareKind) -> Result<HardwareSession, CodecError> {
    let (available, reason) = probe_kind(kind);
    if !available {
        return Err(CodecError::Unavailable(reason));
    }
    let encoder = kind.ffmpeg_encoder().ok_or(CodecError::Unavailable(
        "this backend has no VP8 ffmpeg name",
    ))?;
    Ok(HardwareSession {
        kind,
        ffmpeg: ffmpeg_path(),
        encoder,
        device: if kind == HardwareKind::Vaapi {
            render_node()
        } else {
            None
        },
    })
}

/// True when `ffmpeg -encoders` lists `name`.
pub fn ffmpeg_encoder_listed(ffmpeg: &Path, name: &str) -> Result<bool, CodecError> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|err| CodecError::Unavailable(unavailable_message(err)))?;
    if !output.status.success() {
        return Ok(false);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.split_whitespace().any(|token| token == name))
}

/// Encode one I420 picture by running `ffmpeg`.
///
/// `extra` is inserted before the encoder name. The child must write the
/// encoded bytes to stdout. A non-zero exit is an encode error.
pub fn encode_i420_with_ffmpeg(
    ffmpeg: &Path,
    encoder: &str,
    picture: &Picture,
    extra: &[String],
) -> Result<Vec<u8>, CodecError> {
    if picture.width < 16 || picture.height < 16 {
        return Err(CodecError::BadDimensions);
    }
    let mut raw = Vec::with_capacity(picture.y.len() + picture.u.len() + picture.v.len());
    raw.extend_from_slice(&picture.y);
    raw.extend_from_slice(&picture.u);
    raw.extend_from_slice(&picture.v);
    let size = format!("{}x{}", picture.width, picture.height);
    let mut command = Command::new(ffmpeg);
    command
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-s")
        .arg(&size)
        .arg("-i")
        .arg("pipe:0");
    for arg in extra {
        command.arg(arg);
    }
    command
        .arg("-frames:v")
        .arg("1")
        .arg("-c:v")
        .arg(encoder)
        .arg("-f")
        .arg("ivf")
        .arg("pipe:1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| CodecError::Unavailable(unavailable_message(err)))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or(CodecError::Encode("ffmpeg stdin was not piped".to_string()))?;
        std::io::Write::write_all(stdin, &raw)
            .map_err(|err| CodecError::Encode(err.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| CodecError::Encode(err.to_string()))?;
    if !output.status.success() || output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodecError::Encode(format!(
            "ffmpeg {encoder} failed: {stderr}"
        )));
    }
    Ok(output.stdout)
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

fn probe_kind(kind: HardwareKind) -> (bool, &'static str) {
    match kind {
        HardwareKind::Nvenc => (false, "nvenc is not linked in this build"),
        HardwareKind::Amf => (false, "amf is not linked in this build"),
        HardwareKind::QuickSync => (false, "qsv is not linked in this build"),
        HardwareKind::Vaapi => vaapi_probe(),
        HardwareKind::VideoToolbox => toolbox_probe(),
        HardwareKind::MediaCodec => mediacodec_probe(),
        HardwareKind::MediaFoundation => mediafoundation_probe(),
    }
}

fn vaapi_probe() -> (bool, &'static str) {
    if !cfg!(feature = "hw-vaapi") {
        return (false, "cargo feature hw-vaapi is off");
    }
    if !cfg!(target_os = "linux") {
        return (false, "VA-API is a Linux feature");
    }
    if render_node().is_none() {
        return (false, "VA-API render node is missing");
    }
    encoder_probe("vp8_vaapi", "ffmpeg has no vp8_vaapi encoder")
}

fn toolbox_probe() -> (bool, &'static str) {
    if !cfg!(feature = "hw-videotoolbox") {
        return (false, "cargo feature hw-videotoolbox is off");
    }
    if !cfg!(target_os = "macos") {
        return (false, "VideoToolbox is a macOS feature");
    }
    encoder_probe("vp8_videotoolbox", "ffmpeg has no vp8_videotoolbox encoder")
}

fn mediacodec_probe() -> (bool, &'static str) {
    if !cfg!(feature = "hw-mediacodec") {
        return (false, "cargo feature hw-mediacodec is off");
    }
    if !cfg!(target_os = "android") {
        return (false, "MediaCodec is an Android feature");
    }
    encoder_probe("vp8_mediacodec", "ffmpeg has no vp8_mediacodec encoder")
}

fn mediafoundation_probe() -> (bool, &'static str) {
    if !cfg!(feature = "hw-mediafoundation") {
        return (false, "cargo feature hw-mediafoundation is off");
    }
    if !cfg!(target_os = "windows") {
        return (false, "Media Foundation is a Windows feature");
    }
    encoder_probe("vp8_mf", "ffmpeg has no vp8_mf encoder")
}

fn encoder_probe(name: &str, missing: &'static str) -> (bool, &'static str) {
    match ffmpeg_encoder_listed(&ffmpeg_path(), name) {
        Ok(true) => (true, "ffmpeg lists the VP8 encoder"),
        Ok(false) => (false, missing),
        Err(_) => (false, "ffmpeg is not installed"),
    }
}

fn ffmpeg_path() -> PathBuf {
    std::env::var_os("CBDESK_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
        })
}

fn render_node() -> Option<PathBuf> {
    let entries = std::fs::read_dir("/dev/dri").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if text.starts_with("renderD") {
            return Some(entry.path());
        }
    }
    None
}

fn unavailable_message(err: std::io::Error) -> &'static str {
    let _ = err;
    "ffmpeg is not installed"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hardware_features_fail_closed_until_a_device_and_encoder_exist() {
        for probe in probe_hardware() {
            if probe.available {
                assert!(open_hardware(probe.kind).is_ok());
            } else {
                assert!(open_hardware(probe.kind).is_err(), "{}", probe.reason);
            }
        }
        let vaapi = probe_hardware()
            .into_iter()
            .find(|probe| probe.kind == HardwareKind::Vaapi)
            .expect("vaapi");
        if cfg!(feature = "hw-vaapi") {
            assert!(
                vaapi.available
                    || vaapi.reason.contains("VA-API")
                    || vaapi.reason.contains("ffmpeg")
                    || vaapi.reason.contains("vp8_vaapi"),
                "{}",
                vaapi.reason
            );
        } else {
            assert!(vaapi.reason.contains("hw-vaapi"), "{}", vaapi.reason);
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

    #[cfg(unix)]
    #[test]
    fn a_stub_ffmpeg_lists_vp8_and_returns_bytes() {
        let dir = tempfile_dir();
        let path = dir.join("ffmpeg");
        let mut file = std::fs::File::create(&path).expect("script");
        file.write_all(
            b"#!/bin/sh\n\
              case \" $* \" in\n\
              *\\ -encoders\\ *) printf '%s\\n' ' V..... vp8_vaapi'; exit 0 ;;\n\
              esac\n\
              cat >/dev/null\n\
              printf '%s' 'cbdesk-ivf'\n",
        )
        .expect("write");
        drop(file);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).expect("mode");
        }
        assert!(ffmpeg_encoder_listed(&path, "vp8_vaapi").expect("list"));
        assert!(!ffmpeg_encoder_listed(&path, "vp8_mf").expect("missing"));
        let picture = Picture {
            width: 16,
            height: 16,
            y: vec![20; 256],
            u: vec![128; 64],
            v: vec![128; 64],
        };
        let bytes = encode_i420_with_ffmpeg(&path, "vp8_vaapi", &picture, &[]).expect("encode");
        assert_eq!(bytes, b"cbdesk-ivf");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cbdesk-ffmpeg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }
}

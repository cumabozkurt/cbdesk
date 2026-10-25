// SPDX-License-Identifier: AGPL-3.0-or-later

//! Resolution profiles and independent VP8 strip keyframes.
//!
//! [`FrameProfile::Balanced`] and [`FrameProfile::Preview`] box-filter the
//! picture before one encode. [`encode_strips`] splits the picture into
//! horizontal bands and encodes each band as its own keyframe. Bands do not
//! share a VP8 reference, so they can run at the same time. The live frame
//! header has no full-canvas size, so a host that sends those bands as
//! region updates cannot place them. The CLI `full` profile does not scale.
//! None of these paths is a 30 frame-per-second claim.

use std::thread;
use std::time::Instant;

use cbdesk_capture::Picture;

use crate::damage::{crop_at, paint_at};
use crate::vp8::{vp8_is_inter, Vp8Encoder, Vp8StreamDecoder};
use crate::CodecError;

/// How a screen picture is reduced before the software encoder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameProfile {
    /// Captured even size. The quantizer still comes from the pacer.
    #[default]
    Full,
    /// Half the width and height, quantizer 48.
    Balanced,
    /// Quarter the width and height, quantizer 64.
    Preview,
}

impl FrameProfile {
    /// 1, 2, or 4.
    pub fn downsample(self) -> u32 {
        match self {
            Self::Full => 1,
            Self::Balanced => 2,
            Self::Preview => 4,
        }
    }

    /// Quantizer used when this profile replaces the pacer.
    ///
    /// [`FrameProfile::Full`] still lets the pacer choose. 32 is the index
    /// the 1080p measurement used, not a promise that the pacer stays there.
    pub fn qindex(self) -> u8 {
        match self {
            Self::Full => 32,
            Self::Balanced => 48,
            Self::Preview => 64,
        }
    }

    /// `full`, `balanced`, or `preview`.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "full" => Some(Self::Full),
            "balanced" => Some(Self::Balanced),
            "preview" => Some(Self::Preview),
            _ => None,
        }
    }
}

/// One horizontal band encoded as a VP8 keyframe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripPacket {
    /// Luma y of this band on the picture that was split.
    pub origin_y: u32,
    /// Band width. This is the picture width, not a horizontal crop.
    pub width: u32,
    /// Band height.
    pub height: u32,
    /// VP8 keyframe bytes.
    pub bytes: Vec<u8>,
}

/// Every band of one picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripSet {
    /// Picture width.
    pub width: u32,
    /// Picture height. The bands stack to this height.
    pub height: u32,
    /// Quantizer passed to each band.
    pub qindex: u8,
    /// True when the bands were encoded on separate threads.
    pub parallel: bool,
    /// Wall time of the encode, milliseconds.
    pub wall_ms: u128,
    /// Bands from top to bottom.
    pub packets: Vec<StripPacket>,
}

/// One measured encode. The name is a fixed label, not a user string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathTiming {
    /// Which path this row is.
    pub name: &'static str,
    /// Encoded width.
    pub encoded_width: u32,
    /// Encoded height of one packet, or the scaled picture.
    pub encoded_height: u32,
    /// How many packets were produced.
    pub strips: usize,
    /// Whether strip encodes overlapped.
    pub parallel: bool,
    /// Wall time, milliseconds.
    pub wall_ms: u128,
    /// Sum of packet sizes.
    pub bytes: usize,
}

/// Scale `picture` the way [`FrameProfile`] asks.
///
/// Full size is a clone. Balanced and preview crop the bottom and right
/// edges so both axes are multiples of twice the factor, which keeps the
/// scaled size even, then box-filter. A screen such as 1366×768 can be
/// previewed. A picture that would shrink below 16 pixels is rejected.
pub fn scale_profile(picture: &Picture, profile: FrameProfile) -> Result<Picture, CodecError> {
    let factor = profile.downsample();
    if factor == 1 {
        return Ok(picture.clone());
    }
    let step = factor.saturating_mul(2);
    let width = picture.width - (picture.width % step);
    let height = picture.height - (picture.height % step);
    let cropped = if width == picture.width && height == picture.height {
        picture.clone()
    } else {
        crop_at(picture, 0, 0, width, height)?
    };
    scale_box(&cropped, factor)
}

/// Box-filter `picture` by `factor` on each axis.
///
/// `factor` is 1, 2, or 4. The result is even and at least 16 pixels on
/// each side. A factor of 1 clones the picture.
pub fn scale_box(picture: &Picture, factor: u32) -> Result<Picture, CodecError> {
    if factor == 1 {
        return Ok(picture.clone());
    }
    if factor != 2 && factor != 4 {
        return Err(CodecError::BadDimensions);
    }
    check_picture(picture)?;
    if !picture.width.is_multiple_of(factor) || !picture.height.is_multiple_of(factor) {
        return Err(CodecError::BadDimensions);
    }
    let width = picture.width / factor;
    let height = picture.height / factor;
    if width < 16 || height < 16 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    Ok(Picture {
        width,
        height,
        y: box_plane(&picture.y, picture.width, picture.height, factor)?,
        u: box_plane(&picture.u, picture.width / 2, picture.height / 2, factor)?,
        v: box_plane(&picture.v, picture.width / 2, picture.height / 2, factor)?,
    })
}

/// Encode `picture` as `strips` horizontal VP8 keyframes.
///
/// Each band is at least 16 pixels tall and even. `strips` is reduced when
/// the height cannot hold that many bands. `parallel` uses one thread per
/// band when there is more than one band. The threads do not share a
/// reference frame.
pub fn encode_strips(
    picture: &Picture,
    qindex: u8,
    strips: u32,
    parallel: bool,
) -> Result<StripSet, CodecError> {
    check_picture(picture)?;
    if strips == 0 {
        return Err(CodecError::BadDimensions);
    }
    let spans = strip_spans(picture.height, strips)?;
    let started = Instant::now();
    let packets = if parallel && spans.len() > 1 {
        encode_parallel(picture, qindex, &spans)?
    } else {
        let mut packets = Vec::with_capacity(spans.len());
        for &(origin_y, height) in &spans {
            packets.push(encode_one(picture, origin_y, height, qindex)?);
        }
        packets
    };
    Ok(StripSet {
        width: picture.width,
        height: picture.height,
        qindex,
        parallel: parallel && spans.len() > 1,
        wall_ms: started.elapsed().as_millis(),
        packets,
    })
}

/// Decode every band onto a blank picture of [`StripSet::width`] by
/// [`StripSet::height`].
pub fn composite_strips(set: &StripSet) -> Result<Picture, CodecError> {
    let mut canvas = blank(set.width, set.height)?;
    let mut covered = 0_u32;
    for packet in &set.packets {
        if packet.origin_y != covered || packet.width != set.width {
            return Err(CodecError::BadDimensions);
        }
        let mut decoder = Vp8StreamDecoder::new();
        let region = decoder.decode(&packet.bytes)?;
        if region.width != packet.width || region.height != packet.height {
            return Err(CodecError::BadDimensions);
        }
        paint_at(&mut canvas, &region, 0, packet.origin_y)?;
        covered = covered.saturating_add(packet.height);
    }
    if covered != set.height {
        return Err(CodecError::BadDimensions);
    }
    Ok(canvas)
}

/// Time a solid 1920×1080 picture on the full, strip, and scaled paths.
///
/// This is one synthetic picture. It does not capture a desktop and it does
/// not claim 30 frames per second.
pub fn measure_1080p_paths() -> Result<Vec<PathTiming>, CodecError> {
    let picture = solid(1920, 1080, 40)?;
    let mut rows = Vec::new();
    let started = Instant::now();
    let mut encoder = crate::DamageEncoder::new();
    let full = encoder.encode(&picture, FrameProfile::Full.qindex())?;
    rows.push(PathTiming {
        name: "full-keyframe",
        encoded_width: full.width,
        encoded_height: full.height,
        strips: 1,
        parallel: false,
        wall_ms: started.elapsed().as_millis(),
        bytes: full.bytes.len(),
    });
    rows.push(time_strips(&picture, 4, false)?);
    rows.push(time_strips(&picture, 4, true)?);
    rows.push(time_scaled(&picture, FrameProfile::Balanced)?);
    rows.push(time_scaled(&picture, FrameProfile::Preview)?);
    Ok(rows)
}

fn time_strips(picture: &Picture, strips: u32, parallel: bool) -> Result<PathTiming, CodecError> {
    let set = encode_strips(picture, FrameProfile::Full.qindex(), strips, parallel)?;
    let bytes = set.packets.iter().map(|packet| packet.bytes.len()).sum();
    let height = set.packets.first().map(|packet| packet.height).unwrap_or(0);
    Ok(PathTiming {
        name: if parallel {
            "strips-parallel"
        } else {
            "strips-serial"
        },
        encoded_width: set.width,
        encoded_height: height,
        strips: set.packets.len(),
        parallel: set.parallel,
        wall_ms: set.wall_ms,
        bytes,
    })
}

fn time_scaled(picture: &Picture, profile: FrameProfile) -> Result<PathTiming, CodecError> {
    let scaled = scale_box(picture, profile.downsample())?;
    let started = Instant::now();
    let mut encoder = Vp8Encoder::new();
    let (bytes, keyframe) = encoder.encode(&scaled, profile.qindex())?;
    if !keyframe {
        return Err(CodecError::Encode(
            "scaled picture was not a keyframe".to_string(),
        ));
    }
    Ok(PathTiming {
        name: match profile {
            FrameProfile::Full => "scaled-full",
            FrameProfile::Balanced => "balanced",
            FrameProfile::Preview => "preview",
        },
        encoded_width: scaled.width,
        encoded_height: scaled.height,
        strips: 1,
        parallel: false,
        wall_ms: started.elapsed().as_millis(),
        bytes: bytes.len(),
    })
}

fn encode_parallel(
    picture: &Picture,
    qindex: u8,
    spans: &[(u32, u32)],
) -> Result<Vec<StripPacket>, CodecError> {
    let mut encoded = Vec::with_capacity(spans.len());
    let panicked = thread::scope(|scope| {
        let mut joins = Vec::with_capacity(spans.len());
        for &(origin_y, height) in spans {
            joins.push(scope.spawn(move || encode_one(picture, origin_y, height, qindex)));
        }
        for join in joins {
            match join.join() {
                Ok(packet) => encoded.push(packet),
                Err(_) => return true,
            }
        }
        false
    });
    if panicked {
        return Err(CodecError::Encode(
            "a strip encode thread panicked".to_string(),
        ));
    }
    encoded.into_iter().collect()
}

fn encode_one(
    picture: &Picture,
    origin_y: u32,
    height: u32,
    qindex: u8,
) -> Result<StripPacket, CodecError> {
    let cropped = crop_at(picture, 0, origin_y, picture.width, height)?;
    let mut encoder = Vp8Encoder::new();
    let (bytes, keyframe) = encoder.encode(&cropped, qindex)?;
    if !keyframe || vp8_is_inter(&bytes) {
        return Err(CodecError::Encode("strip was not a keyframe".to_string()));
    }
    Ok(StripPacket {
        origin_y,
        width: picture.width,
        height,
        bytes,
    })
}

fn strip_spans(height: u32, want: u32) -> Result<Vec<(u32, u32)>, CodecError> {
    if height < 16 || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    let count = want.min(height / 16).max(1);
    let base = (height / count) & !1;
    if base < 16 {
        return Err(CodecError::BadDimensions);
    }
    let mut spans = Vec::with_capacity(count as usize);
    let mut y = 0_u32;
    for index in 0..count {
        let band = if index + 1 == count { height - y } else { base };
        if band < 16 || !band.is_multiple_of(2) || y.saturating_add(band) > height {
            return Err(CodecError::BadDimensions);
        }
        spans.push((y, band));
        y = y.saturating_add(band);
    }
    if y != height {
        return Err(CodecError::BadDimensions);
    }
    Ok(spans)
}

fn box_plane(plane: &[u8], width: u32, height: u32, factor: u32) -> Result<Vec<u8>, CodecError> {
    let out_w = width / factor;
    let out_h = height / factor;
    let expect = (width as usize).saturating_mul(height as usize);
    if plane.len() != expect || out_w == 0 || out_h == 0 {
        return Err(CodecError::BadDimensions);
    }
    let count = (factor as usize).saturating_mul(factor as usize);
    let mut out = Vec::with_capacity((out_w as usize).saturating_mul(out_h as usize));
    for oy in 0..out_h {
        for ox in 0..out_w {
            let mut sum = 0_u32;
            for row in 0..factor {
                let src_y = oy.saturating_mul(factor).saturating_add(row);
                let start = (src_y as usize)
                    .saturating_mul(width as usize)
                    .saturating_add((ox.saturating_mul(factor)) as usize);
                let end = start.saturating_add(factor as usize);
                let Some(slice) = plane.get(start..end) else {
                    return Err(CodecError::BadDimensions);
                };
                for sample in slice {
                    sum = sum.saturating_add(u32::from(*sample));
                }
            }
            let value = u8::try_from(sum / count as u32).unwrap_or(255);
            out.push(value);
        }
    }
    Ok(out)
}

fn check_picture(picture: &Picture) -> Result<(), CodecError> {
    let luma = (picture.width as usize).saturating_mul(picture.height as usize);
    let chroma = (picture.width as usize / 2).saturating_mul(picture.height as usize / 2);
    if picture.width < 16
        || picture.height < 16
        || !picture.width.is_multiple_of(2)
        || !picture.height.is_multiple_of(2)
        || picture.y.len() != luma
        || picture.u.len() != chroma
        || picture.v.len() != chroma
    {
        return Err(CodecError::BadDimensions);
    }
    Ok(())
}

fn blank(width: u32, height: u32) -> Result<Picture, CodecError> {
    solid(width, height, 0)
}

fn solid(width: u32, height: u32, luma: u8) -> Result<Picture, CodecError> {
    if width < 16 || height < 16 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    let pixels = (width as usize).saturating_mul(height as usize);
    let chroma = (width as usize / 2).saturating_mul(height as usize / 2);
    Ok(Picture {
        width,
        height,
        y: vec![luma; pixels],
        u: vec![128; chroma],
        v: vec![128; chroma],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_parse_and_reject_other_names() {
        assert_eq!(FrameProfile::parse("full"), Some(FrameProfile::Full));
        assert_eq!(
            FrameProfile::parse("balanced"),
            Some(FrameProfile::Balanced)
        );
        assert_eq!(FrameProfile::parse("preview"), Some(FrameProfile::Preview));
        assert_eq!(FrameProfile::parse("fast"), None);
        assert_eq!(FrameProfile::default().downsample(), 1);
    }

    #[test]
    fn box_filter_halves_a_checker_and_rejects_a_factor_of_three() {
        let mut picture = solid(32, 32, 0).expect("solid");
        picture.y[..16 * 32].fill(255);
        let half = scale_box(&picture, 2).expect("half");
        assert_eq!((half.width, half.height), (16, 16));
        assert_eq!(half.y[0], 255, "top {}", half.y[0]);
        assert_eq!(half.y[8 * 16], 0, "bottom {}", half.y[8 * 16]);
        assert_eq!(
            scale_box(&picture, 3).expect_err("three"),
            CodecError::BadDimensions
        );
        let odd = solid(70, 36, 9).expect("odd");
        let fitted = scale_profile(&odd, FrameProfile::Balanced).expect("fit");
        assert_eq!((fitted.width, fitted.height), (34, 18));
        let tiny = solid(16, 16, 1).expect("tiny");
        assert!(scale_box(&tiny, 2).is_err());
    }

    #[test]
    fn two_strips_round_trip_a_bright_lower_band() {
        let mut picture = solid(64, 64, 20).expect("base");
        for row in 32..64 {
            let start = (row as usize) * 64;
            picture.y[start..start + 64].fill(240);
        }
        let set = encode_strips(&picture, 32, 2, true).expect("strips");
        assert!(set.parallel);
        assert_eq!(set.packets.len(), 2);
        assert_eq!(set.packets[0].origin_y, 0);
        assert_eq!(set.packets[1].origin_y, 32);
        assert!(set.packets.iter().all(|packet| packet.height == 32));
        let placed = composite_strips(&set).expect("composite");
        assert!(placed.y[0] < 80, "top {}", placed.y[0]);
        assert!(placed.y[32 * 64] > 180, "bottom {}", placed.y[32 * 64]);
        assert!(encode_strips(&picture, 32, 0, false).is_err());
    }

    #[test]
    #[ignore = "refresh docs/dev/media-measurement.md; 1080p encodes are slow"]
    fn vp8_1080p_profiles() {
        let handle = thread::Builder::new()
            .name("vp8-profiles".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| measure_1080p_paths().expect("1080p"))
            .expect("spawn");
        let rows = handle.join().expect("join");
        assert!(rows.len() >= 5);
        for row in &rows {
            assert!(row.bytes > 0, "{}", row.name);
            assert!(row.encoded_width >= 16 && row.encoded_height >= 16);
            eprintln!(
                "profile {} {}x{} strips={} parallel={} wall_ms={} bytes={}",
                row.name,
                row.encoded_width,
                row.encoded_height,
                row.strips,
                row.parallel,
                row.wall_ms,
                row.bytes
            );
        }
        let balanced = rows
            .iter()
            .find(|row| row.name == "balanced")
            .expect("balanced");
        assert_eq!(
            (balanced.encoded_width, balanced.encoded_height),
            (960, 540)
        );
        let preview = rows
            .iter()
            .find(|row| row.name == "preview")
            .expect("preview");
        assert_eq!((preview.encoded_width, preview.encoded_height), (480, 270));
        let parallel = rows
            .iter()
            .find(|row| row.name == "strips-parallel")
            .expect("parallel");
        assert_eq!(parallel.strips, 4);
        assert!(parallel.parallel);
        assert_eq!(parallel.encoded_width, 1920);
    }
}

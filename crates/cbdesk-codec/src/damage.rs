// SPDX-License-Identifier: AGPL-3.0-or-later

//! Damage tracking for the software VP8 path.
//!
//! The scan compares 64×64 tiles on two threads when the picture is tall
//! enough. Each 16-byte group is a [`wide::u8x16`] compare (SSE2 on x86_64,
//! NEON on aarch64). The tail is scalar. This crate still has no `unsafe`
//! block. An unchanged
//! picture is not encoded. A dirty region that covers at most a quarter of the
//! picture is cropped, aligned to 16 pixels, and encoded as its own keyframe.
//! Anything larger stays on the full-frame inter encoder.

use std::thread;
use std::time::Instant;

use cbdesk_capture::Picture;

use crate::error::CodecError;
use crate::vp8::{Vp8Encoder, Vp8StreamDecoder};

const TILE: u32 = 64;

/// One encoded step and where it belongs on the full picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamagePacket {
    /// VP8 bytes. Empty when [`DamagePacket::unchanged`] is set.
    pub bytes: Vec<u8>,
    /// True when a decoder can start from `bytes` without an earlier packet.
    pub keyframe: bool,
    /// The picture matched the previous one. Nothing was encoded.
    pub unchanged: bool,
    /// Luma x of a region packet. Zero for a full frame.
    pub origin_x: u32,
    /// Luma y of a region packet.
    pub origin_y: u32,
    /// Width of this packet's picture.
    pub width: u32,
    /// Height of this packet's picture.
    pub height: u32,
    /// True when `width` and `height` are a crop, not the full picture.
    pub region: bool,
    /// How long the tile scan took, in microseconds.
    pub scan_us: u128,
}

/// Wall times for one keyframe, one unchanged repeat, and one small region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageTiming {
    /// Picture width.
    pub width: u32,
    /// Picture height.
    pub height: u32,
    /// Keyframe encode, milliseconds.
    pub full_ms: u128,
    /// Repeat of the same picture, milliseconds. This does not encode.
    pub unchanged_ms: u128,
    /// Region encode, milliseconds.
    pub region_ms: u128,
    /// Encoded region width.
    pub region_width: u32,
    /// Encoded region height.
    pub region_height: u32,
    /// Region packet size.
    pub region_bytes: usize,
    /// Tile scan of the region frame, microseconds.
    pub scan_us: u128,
}

/// Full-frame encoder plus the previous picture used for the tile scan.
#[derive(Debug)]
pub struct DamageEncoder {
    stream: Vp8Encoder,
    previous: Option<Picture>,
}

impl DamageEncoder {
    /// An encoder that has not seen a picture yet.
    pub fn new() -> Self {
        Self {
            stream: Vp8Encoder::new(),
            previous: None,
        }
    }

    /// Encode `picture` or report that it matches the previous one.
    pub fn encode(&mut self, picture: &Picture, qindex: u8) -> Result<DamagePacket, CodecError> {
        let started = Instant::now();
        let damage = match &self.previous {
            Some(previous) => scan_damage(previous, picture)?,
            None => None,
        };
        let scan_us = started.elapsed().as_micros();
        if self.previous.is_some() && damage.is_none() {
            return Ok(DamagePacket {
                bytes: Vec::new(),
                keyframe: false,
                unchanged: true,
                origin_x: 0,
                origin_y: 0,
                width: picture.width,
                height: picture.height,
                region: false,
                scan_us,
            });
        }
        let full_area = u64::from(picture.width).saturating_mul(u64::from(picture.height));
        if let Some(rect) = damage {
            let area = u64::from(rect.width).saturating_mul(u64::from(rect.height));
            let small = area.saturating_mul(4) <= full_area
                && rect.width < picture.width
                && rect.height < picture.height;
            if small {
                let cropped = crop(picture, rect)?;
                let mut once = Vp8Encoder::new();
                let (bytes, keyframe) = once.encode(&cropped, qindex)?;
                if !keyframe {
                    return Err(CodecError::Encode(
                        "region packet was not a keyframe".to_string(),
                    ));
                }
                self.previous = Some(picture.clone());
                return Ok(DamagePacket {
                    bytes,
                    keyframe: true,
                    unchanged: false,
                    origin_x: rect.x,
                    origin_y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    region: true,
                    scan_us,
                });
            }
        }
        let (bytes, keyframe) = self.stream.encode(picture, qindex)?;
        self.previous = Some(picture.clone());
        Ok(DamagePacket {
            bytes,
            keyframe,
            unchanged: false,
            origin_x: 0,
            origin_y: 0,
            width: picture.width,
            height: picture.height,
            region: false,
            scan_us,
        })
    }
}

impl Default for DamageEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Header fields a viewer needs in order to place one VP8 payload.
#[derive(Clone, Copy, Debug)]
pub struct PlacedFrame<'a> {
    /// The payload is empty and the previous picture still stands.
    pub unchanged: bool,
    /// Luma x. Ignored unless `damage_width` is non-zero.
    pub damage_x: u32,
    /// Luma y.
    pub damage_y: u32,
    /// Region width. Zero means `payload` is a full frame.
    pub damage_width: u32,
    /// Region height.
    pub damage_height: u32,
    /// VP8 bytes.
    pub payload: &'a [u8],
}

/// Decode `frame` onto `canvas`.
///
/// A region payload uses a fresh decoder so the full-frame `stream` keeps its
/// reference. An unchanged frame returns the canvas that is already there.
pub fn composite_vp8(
    canvas: &mut Option<Picture>,
    stream: &mut Vp8StreamDecoder,
    frame: PlacedFrame<'_>,
) -> Result<Picture, CodecError> {
    if frame.unchanged {
        return canvas.clone().ok_or(CodecError::NoFrame);
    }
    if frame.damage_width > 0 && frame.damage_height > 0 {
        let mut once = Vp8StreamDecoder::new();
        let region = once.decode(frame.payload)?;
        if region.width != frame.damage_width || region.height != frame.damage_height {
            return Err(CodecError::BadDimensions);
        }
        let base = canvas.as_mut().ok_or(CodecError::NoFrame)?;
        paint_region(base, &region, frame.damage_x, frame.damage_y)?;
        return Ok(base.clone());
    }
    let picture = stream.decode(frame.payload)?;
    *canvas = Some(picture.clone());
    Ok(picture)
}

/// Allocate a blank canvas when a strip header names one.
///
/// A width or height of zero leaves `canvas` alone. That is how a header
/// written before canvas fields existed behaves. A non-zero size replaces
/// the canvas when it is missing, a different size, or `strip_index` is
/// zero, which starts a new strip set. The blank picture is luma 0 and
/// chroma 128. Both axes must be even and at least 16.
pub fn ensure_strip_canvas(
    canvas: &mut Option<Picture>,
    width: u32,
    height: u32,
    strip_index: u32,
) -> Result<(), CodecError> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let replace = match canvas.as_ref() {
        None => true,
        Some(picture) => strip_index == 0 || picture.width != width || picture.height != height,
    };
    if replace {
        *canvas = Some(blank_canvas(width, height)?);
    }
    Ok(())
}

fn blank_canvas(width: u32, height: u32) -> Result<Picture, CodecError> {
    if width < 16 || height < 16 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    let pixels = (width as usize).saturating_mul(height as usize);
    let chroma = (width as usize / 2).saturating_mul(height as usize / 2);
    Ok(Picture {
        width,
        height,
        y: vec![0; pixels],
        u: vec![128; chroma],
        v: vec![128; chroma],
    })
}

/// Time one  keyframe, one unchanged repeat, and one 16×16 dirty block.
pub fn measure_damage_path(
    width: u32,
    height: u32,
    qindex: u8,
) -> Result<DamageTiming, CodecError> {
    let base = solid(width, height, 40)?;
    let mut encoder = DamageEncoder::new();
    let started = Instant::now();
    let full = encoder.encode(&base, qindex)?;
    let full_ms = started.elapsed().as_millis();
    if !full.keyframe || full.region || full.unchanged {
        return Err(CodecError::Encode(
            "the first damage measurement was not a full keyframe".to_string(),
        ));
    }
    let started = Instant::now();
    let same = encoder.encode(&base, qindex)?;
    let unchanged_ms = started.elapsed().as_millis();
    if !same.unchanged || !same.bytes.is_empty() {
        return Err(CodecError::Encode(
            "an unchanged picture was encoded".to_string(),
        ));
    }
    let mut dirty = base;
    fill_block(&mut dirty, 16, 16, 16, 255)?;
    let started = Instant::now();
    let region = encoder.encode(&dirty, qindex)?;
    let region_ms = started.elapsed().as_millis();
    if !region.region {
        return Err(CodecError::Encode(
            "a 16 pixel block was encoded as a full frame".to_string(),
        ));
    }
    Ok(DamageTiming {
        width,
        height,
        full_ms,
        unchanged_ms,
        region_ms,
        region_width: region.width,
        region_height: region.height,
        region_bytes: region.bytes.len(),
        scan_us: region.scan_us,
    })
}

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct Partial {
    dirty: bool,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

fn scan_damage(previous: &Picture, next: &Picture) -> Result<Option<Rect>, CodecError> {
    if previous.width != next.width || previous.height != next.height {
        return Ok(Some(Rect {
            x: 0,
            y: 0,
            width: next.width,
            height: next.height,
        }));
    }
    check_planes(previous)?;
    check_planes(next)?;
    let width = next.width;
    let height = next.height;
    let tiles_y = height.div_ceil(TILE);
    let split = tiles_y / 2;
    let partial = if tiles_y >= 2 {
        thread::scope(|scope| {
            let top = scope.spawn(|| scan_rows(previous, next, 0, split * TILE));
            let bottom = scan_rows(previous, next, split * TILE, height);
            match top.join() {
                Ok(top) => Ok(merge(top, bottom)),
                Err(_) => Err(CodecError::Encode("damage scan panicked".to_string())),
            }
        })?
    } else {
        scan_rows(previous, next, 0, height)
    };
    if !partial.dirty {
        return Ok(None);
    }
    Ok(Some(align_rect(partial, width, height)))
}

fn scan_rows(previous: &Picture, next: &Picture, y0: u32, y1: u32) -> Partial {
    let width = next.width;
    let mut partial = Partial {
        dirty: false,
        min_x: width,
        min_y: next.height,
        max_x: 0,
        max_y: 0,
    };
    let mut y = y0;
    while y < y1 {
        let y_end = (y + TILE).min(y1);
        let mut x = 0;
        while x < width {
            let x_end = (x + TILE).min(width);
            if tile_differs(previous, next, x, y, x_end, y_end) {
                partial.dirty = true;
                partial.min_x = partial.min_x.min(x);
                partial.min_y = partial.min_y.min(y);
                partial.max_x = partial.max_x.max(x_end);
                partial.max_y = partial.max_y.max(y_end);
            }
            x = x_end;
        }
        y = y_end;
    }
    partial
}

fn tile_differs(previous: &Picture, next: &Picture, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
    plane_differs(&previous.y, &next.y, next.width, x0, y0, x1, y1)
        || chroma_differs(previous, next, x0, y0, x1, y1)
}

fn chroma_differs(previous: &Picture, next: &Picture, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
    let width = next.width / 2;
    plane_differs(
        &previous.u,
        &next.u,
        width,
        x0 / 2,
        y0 / 2,
        x1.div_ceil(2),
        y1.div_ceil(2),
    ) || plane_differs(
        &previous.v,
        &next.v,
        width,
        x0 / 2,
        y0 / 2,
        x1.div_ceil(2),
        y1.div_ceil(2),
    )
}

fn plane_differs(
    previous: &[u8],
    next: &[u8],
    stride: u32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
) -> bool {
    let stride = stride as usize;
    let x0 = x0 as usize;
    let x1 = x1 as usize;
    for y in y0..y1 {
        let start = (y as usize).saturating_mul(stride).saturating_add(x0);
        let end = (y as usize).saturating_mul(stride).saturating_add(x1);
        let Some(left) = previous.get(start..end) else {
            return true;
        };
        let Some(right) = next.get(start..end) else {
            return true;
        };
        if chunk_differs(left, right) {
            return true;
        }
    }
    false
}

fn chunk_differs(left: &[u8], right: &[u8]) -> bool {
    crate::cmp::bytes_differ(left, right)
}

pub(crate) fn crop_at(
    picture: &Picture,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<Picture, CodecError> {
    crop(
        picture,
        Rect {
            x,
            y,
            width,
            height,
        },
    )
}

pub(crate) fn paint_at(
    base: &mut Picture,
    region: &Picture,
    x: u32,
    y: u32,
) -> Result<(), CodecError> {
    paint_region(base, region, x, y)
}

fn merge(top: Partial, bottom: Partial) -> Partial {
    if !top.dirty {
        return bottom;
    }
    if !bottom.dirty {
        return top;
    }
    Partial {
        dirty: true,
        min_x: top.min_x.min(bottom.min_x),
        min_y: top.min_y.min(bottom.min_y),
        max_x: top.max_x.max(bottom.max_x),
        max_y: top.max_y.max(bottom.max_y),
    }
}

fn align_rect(partial: Partial, width: u32, height: u32) -> Rect {
    let x = partial.min_x - (partial.min_x % 16);
    let y = partial.min_y - (partial.min_y % 16);
    let mut x1 = partial.max_x.div_ceil(16).saturating_mul(16);
    let mut y1 = partial.max_y.div_ceil(16).saturating_mul(16);
    if x1 < x.saturating_add(16) {
        x1 = x.saturating_add(16);
    }
    if y1 < y.saturating_add(16) {
        y1 = y.saturating_add(16);
    }
    let x1 = x1.min(width);
    let y1 = y1.min(height);
    Rect {
        x,
        y,
        width: x1.saturating_sub(x),
        height: y1.saturating_sub(y),
    }
}

fn crop(picture: &Picture, rect: Rect) -> Result<Picture, CodecError> {
    if !rect.width.is_multiple_of(2) || !rect.height.is_multiple_of(2) {
        return Err(CodecError::BadDimensions);
    }
    Ok(Picture {
        width: rect.width,
        height: rect.height,
        y: crop_plane(
            &picture.y,
            picture.width,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
        )?,
        u: crop_plane(
            &picture.u,
            picture.width / 2,
            rect.x / 2,
            rect.y / 2,
            rect.width / 2,
            rect.height / 2,
        )?,
        v: crop_plane(
            &picture.v,
            picture.width / 2,
            rect.x / 2,
            rect.y / 2,
            rect.width / 2,
            rect.height / 2,
        )?,
    })
}

fn crop_plane(
    plane: &[u8],
    stride: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CodecError> {
    let stride = stride as usize;
    let mut out = Vec::with_capacity((width as usize).saturating_mul(height as usize));
    for row in y..y.saturating_add(height) {
        let start = (row as usize)
            .saturating_mul(stride)
            .saturating_add(x as usize);
        let end = start.saturating_add(width as usize);
        let Some(slice) = plane.get(start..end) else {
            return Err(CodecError::BadDimensions);
        };
        out.extend_from_slice(slice);
    }
    Ok(out)
}

fn paint_region(base: &mut Picture, region: &Picture, x: u32, y: u32) -> Result<(), CodecError> {
    if x.saturating_add(region.width) > base.width || y.saturating_add(region.height) > base.height
    {
        return Err(CodecError::BadDimensions);
    }
    paint_plane(PlaneCopy {
        base: &mut base.y,
        stride: base.width,
        src: &region.y,
        src_stride: region.width,
        x,
        y,
        width: region.width,
        height: region.height,
    })?;
    paint_plane(PlaneCopy {
        base: &mut base.u,
        stride: base.width / 2,
        src: &region.u,
        src_stride: region.width / 2,
        x: x / 2,
        y: y / 2,
        width: region.width / 2,
        height: region.height / 2,
    })?;
    paint_plane(PlaneCopy {
        base: &mut base.v,
        stride: base.width / 2,
        src: &region.v,
        src_stride: region.width / 2,
        x: x / 2,
        y: y / 2,
        width: region.width / 2,
        height: region.height / 2,
    })?;
    Ok(())
}

struct PlaneCopy<'a> {
    base: &'a mut [u8],
    stride: u32,
    src: &'a [u8],
    src_stride: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn paint_plane(plane: PlaneCopy<'_>) -> Result<(), CodecError> {
    let stride = plane.stride as usize;
    let src_stride = plane.src_stride as usize;
    for row in 0..plane.height {
        let dest_start = ((plane.y.saturating_add(row)) as usize)
            .saturating_mul(stride)
            .saturating_add(plane.x as usize);
        let dest_end = dest_start.saturating_add(plane.width as usize);
        let src_start = (row as usize).saturating_mul(src_stride);
        let src_end = src_start.saturating_add(plane.width as usize);
        let Some(dest) = plane.base.get_mut(dest_start..dest_end) else {
            return Err(CodecError::BadDimensions);
        };
        let Some(from) = plane.src.get(src_start..src_end) else {
            return Err(CodecError::BadDimensions);
        };
        dest.copy_from_slice(from);
    }
    Ok(())
}

fn check_planes(picture: &Picture) -> Result<(), CodecError> {
    let luma = (picture.width as usize).saturating_mul(picture.height as usize);
    let chroma = (picture.width as usize / 2).saturating_mul(picture.height as usize / 2);
    if picture.y.len() != luma || picture.u.len() != chroma || picture.v.len() != chroma {
        return Err(CodecError::BadDimensions);
    }
    if picture.width < 16 || picture.height < 16 {
        return Err(CodecError::BadDimensions);
    }
    Ok(())
}

fn solid(width: u32, height: u32, luma: u8) -> Result<Picture, CodecError> {
    if width < 32 || height < 32 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
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

fn fill_block(
    picture: &mut Picture,
    x: u32,
    y: u32,
    size: u32,
    value: u8,
) -> Result<(), CodecError> {
    for row in y..y.saturating_add(size) {
        let start = (row as usize)
            .saturating_mul(picture.width as usize)
            .saturating_add(x as usize);
        let end = start.saturating_add(size as usize);
        let Some(slice) = picture.y.get_mut(start..end) else {
            return Err(CodecError::BadDimensions);
        };
        slice.fill(value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_picture_is_not_encoded_and_a_block_is_a_region() {
        let timing = measure_damage_path(320, 240, 48).expect("measure");
        assert!(timing.region_width < timing.width);
        assert!(timing.region_height < timing.height);
        assert!(timing.region_bytes > 0);
        assert!(timing.unchanged_ms <= timing.full_ms);
        assert!(timing.scan_us < 2_000_000);
    }

    #[test]
    fn a_region_decodes_back_onto_the_full_picture() {
        let base = solid(320, 240, 40).expect("base");
        let mut encoder = DamageEncoder::new();
        let full = encoder.encode(&base, 48).expect("key");
        assert!(full.keyframe && !full.region);
        let mut dirty = base.clone();
        fill_block(&mut dirty, 32, 48, 16, 255).expect("block");
        let region = encoder.encode(&dirty, 16).expect("region");
        assert!(region.region);
        let mut canvas = None;
        let mut stream = Vp8StreamDecoder::new();
        composite_vp8(
            &mut canvas,
            &mut stream,
            PlacedFrame {
                unchanged: false,
                damage_x: 0,
                damage_y: 0,
                damage_width: 0,
                damage_height: 0,
                payload: &full.bytes,
            },
        )
        .expect("full");
        let placed = composite_vp8(
            &mut canvas,
            &mut stream,
            PlacedFrame {
                unchanged: false,
                damage_x: region.origin_x,
                damage_y: region.origin_y,
                damage_width: region.width,
                damage_height: region.height,
                payload: &region.bytes,
            },
        )
        .expect("placed");
        let index = (48_usize).saturating_mul(320).saturating_add(32);
        assert!(placed.y[index] > 180, "painted {}", placed.y[index]);
        assert!(placed.y[0] < 80, "background {}", placed.y[0]);
        let again = composite_vp8(
            &mut canvas,
            &mut stream,
            PlacedFrame {
                unchanged: true,
                damage_x: 0,
                damage_y: 0,
                damage_width: 0,
                damage_height: 0,
                payload: &[],
            },
        )
        .expect("unchanged");
        assert!(again.y[index] > 180);
    }

    #[test]
    fn a_strip_canvas_starts_blank_and_a_later_band_keeps_it() {
        let mut canvas = None;
        ensure_strip_canvas(&mut canvas, 0, 0, 0).expect("legacy");
        assert!(canvas.is_none());
        ensure_strip_canvas(&mut canvas, 32, 32, 0).expect("blank");
        let picture = canvas.as_ref().expect("canvas");
        assert_eq!((picture.width, picture.height), (32, 32));
        assert!(picture.y.iter().all(|sample| *sample == 0));
        assert!(picture.u.iter().all(|sample| *sample == 128));
        assert!(picture.v.iter().all(|sample| *sample == 128));
        canvas.as_mut().expect("paint").y[0] = 9;
        ensure_strip_canvas(&mut canvas, 32, 32, 1).expect("keep");
        assert_eq!(canvas.as_ref().expect("kept").y[0], 9);
        ensure_strip_canvas(&mut canvas, 32, 32, 0).expect("reset");
        assert_eq!(canvas.as_ref().expect("reset").y[0], 0);
        assert_eq!(
            ensure_strip_canvas(&mut canvas, 15, 16, 0).expect_err("odd"),
            CodecError::BadDimensions
        );
    }

    #[test]
    fn a_1080p_scan_stays_under_a_second() {
        let width = 1920_u32;
        let height = 1080_u32;
        let pixels = (width as usize) * (height as usize);
        let chroma = (width as usize / 2) * (height as usize / 2);
        let previous = Picture {
            width,
            height,
            y: vec![20; pixels],
            u: vec![128; chroma],
            v: vec![128; chroma],
        };
        let mut next = previous.clone();
        next.y[pixels / 2] = 21;
        let started = Instant::now();
        let damage = scan_damage(&previous, &next).expect("scan");
        assert!(started.elapsed().as_millis() < 1_000);
        let rect = damage.expect("one byte changed");
        assert!(rect.width <= 64);
        assert!(rect.height <= 64);
    }

    #[test]
    #[ignore = "refresh docs/dev/media-measurement.md; pure-Rust 1080p is slow"]
    fn vp8_1080p_damage_path() {
        let handle = thread::Builder::new()
            .name("vp8-damage".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| measure_damage_path(1920, 1080, 32).expect("1080p"))
            .expect("spawn");
        let timing = handle.join().expect("join");
        assert!(timing.unchanged_ms < timing.full_ms);
        assert!(timing.region_width < 1920);
        assert!(timing.region_height < 1080);
        assert!(timing.region_bytes > 0);
        assert!(timing.region_ms < timing.full_ms);
        eprintln!(
            "damage 1080p full_ms={} unchanged_ms={} region_ms={} region={}x{} bytes={} scan_us={}",
            timing.full_ms,
            timing.unchanged_ms,
            timing.region_ms,
            timing.region_width,
            timing.region_height,
            timing.region_bytes,
            timing.scan_us
        );
    }
}

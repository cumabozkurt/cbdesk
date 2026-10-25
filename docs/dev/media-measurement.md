<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Software media measurements

These numbers come from this workspace. They are not a 30 frame-per-second
claim and nobody confirmed they heard the audio.

## Opus loopback

`software_loopback` encodes eight synthetic 20 ms stereo frames at 48 kHz,
holds them in the 40–120 ms jitter buffer, and decodes them with the same
`PacketCodec` a microphone packet uses. The test requires decoded energy
above 0.01 and a mean absolute sample error below 0.2. `device_opened` is
false. cpal is not called. A missing microphone or speaker is a different
test and is an error, not a silent success.

## Software VP8 at 1080p

`measure_vp8(1920, 1080, 2, 32)` encodes one keyframe and one inter frame
with oxideav-vp8. The ignored test `vp8_1080p_two_frames` runs that on a
16 MiB stack. Default `cargo test` does not run it.

Both runs used the ignored test `vp8_1080p_two_frames` on this machine on
2026-10-25. Output size was 29550 bytes, with one keyframe and one inter
frame.

| Build | Elapsed | About |
| --- | --- | --- |
| debug (`unoptimized + debuginfo`) | 19577 ms | 9.8 s per frame |
| release (`--release`) | 416 ms | 208 ms per frame, under 5 frames per second |

Neither result is 30 frames per second. The pacer still targets 33 ms. A
release frame at this size misses that budget and asks the pacer to skip
and coarsen. This is one machine and two synthetic frames, not a soak and
not a camera capture.

## Damage path at 1080p

`measure_damage_path(1920, 1080, 32)` encodes one full keyframe, repeats
that picture, then paints a 16×16 block. The repeat is not encoded. The
block is aligned to a 64×64 tile and encoded as its own keyframe. The
ignored test `vp8_1080p_damage_path` runs that on a 16 MiB stack. Default
`cargo test` does not run it.

One release run on this machine on 2026-10-25 printed:

`damage 1080p full_ms=201 unchanged_ms=0 region_ms=2 region=64x64 bytes=34 scan_us=460`

| Step | Reported time | What it is |
| --- | --- | --- |
| Full keyframe | 201 ms | One 1920×1080 software VP8 keyframe |
| Unchanged repeat | 0 ms | Millisecond timer. The picture was not encoded |
| Region | 2 ms | 64×64 keyframe, 34 bytes |
| Tile scan of the dirty frame | 460 µs | 64×64 tiles, eight bytes at a time, two threads |

The scan uses ordinary integer comparison. This crate forbids unsafe code,
so there are no explicit SIMD intrinsics. A static picture skips the
encoder. A change larger than a quarter of the picture, or as wide or as
tall as the frame, stays on the full-frame inter encoder and still costs
about a fifth of a second at this size. The 2 ms region time is this
synthetic block, not a desktop and not 30 frames per second. The 1080p
test checks the byte count and the times. It does not decode the 34-byte
region. A separate 320×240 test decodes a region and checks one painted
pixel and the background. The two-frame table above is a different test
and is unchanged.

## Hardware encoders

`hw-vaapi`, `hw-mediafoundation`, `hw-videotoolbox`, and `hw-mediacodec`
are cargo features and are off by default. When one is on, the probe asks
`ffmpeg` (or `CBDESK_FFMPEG`) whether `vp8_vaapi`, `vp8_mf`,
`vp8_videotoolbox`, or `vp8_mediacodec` is listed, and whether the device
is present. VA-API also requires a `/dev/dri/renderD*` node. There is no
in-process VA-API, Media Foundation, VideoToolbox, or MediaCodec context.
The quantizer index is not passed to ffmpeg. NVENC, AMF, and Quick Sync
have no feature flag and stay unavailable. A missing feature, binary,
encoder name, or device is `Unavailable`. This pass did not record a
successful hardware encode. Software VP8 is 4:2:0. `require_chroma` of
4:4:4 fails. Quality presets only choose a quantizer index (16, 32, 48, or
a custom index). They are not a visual score.

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

## Hardware encoders

`probe_hardware` reports NVENC, AMF, Quick Sync, VA-API, VideoToolbox,
MediaCodec, and D3D11VA as unavailable. `open_hardware` returns an error.
No hardware encoder is linked. Software VP8 is 4:2:0. `require_chroma` of
4:4:4 fails. Quality presets only choose a quantizer index (16, 32, 48, or
a custom index). They are not a visual score.

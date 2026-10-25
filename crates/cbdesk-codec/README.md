Software VP8 and AV1 for a color picture.

VP8 is oxideav-vp8 0.2.7, MIT, with default features off. It does not link a
system libvpx. The first frame is a keyframe. A later frame of the same size
is a motion-compensated inter frame (`encode_p_frame_multi_ref`, diamond
search). A 48×16 test checks a block that moved 16 pixels at a coarse
quantizer. AV1 is rav1e with assembler and git tooling disabled, and
rav1d-safe under AGPL-3.0-only, and each AV1 call is one still. Negotiation
prefers VP8 when both peers offer it. The live quantizer moves with encode
time and receiver feedback. The synthetic barcode stays at quantizer 0.
A 1080p timing is in `docs/dev/media-measurement.md` and is not 30 frames per second.

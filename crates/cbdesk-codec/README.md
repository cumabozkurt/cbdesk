Software VP8 and AV1 for one grayscale picture.

VP8 is oxideav-vp8 0.2.7, MIT, with default features off. It does not link a
system libvpx. The decoder entry used here accepts keyframes. AV1 is rav1e
with assembler and git tooling disabled, and rav1d-safe under AGPL-3.0-only.
Negotiation prefers VP8 when both peers offer it. Each call encodes one still
picture, so the payload is a keyframe.

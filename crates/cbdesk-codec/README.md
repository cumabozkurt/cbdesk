Software AV1 for one grayscale picture.

The encoder is rav1e with assembler and git tooling disabled. The decoder is
rav1d-safe under AGPL-3.0-only. The advertised capability is `VideoAv1`. VP8
is not linked. Each call encodes one still picture, so the payload is a
keyframe.

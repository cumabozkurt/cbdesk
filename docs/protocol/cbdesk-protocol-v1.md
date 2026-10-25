# Protocol `cbdesk/1`

ALPN token: `cbdesk/1`. Major 1, minor 0. The readable schema is
`proto/cbdesk/v1/session.proto`. The normative encoder for this repository is
`cbdesk-proto`. It is a small protobuf codec, not `prost`, so unknown field
numbers round-trip without a code generator. CI does not install `protoc`.

## Version rule

Major 0 is never compatible. A future build may speak major N with N and N-1.
This build only accepts major 1 (`session_supports`). The helper
`major_compatible` is tested so the N / N-1 rule is not invented later.

## Limits

| Payload | Limit |
| --- | --- |
| Control message | 64 KiB |
| Clipboard text in `ClipboardText` | 60 KiB |
| Clipboard reassembly ceiling | 64 MiB default and absolute cap |
| Video frame | 16 MiB |
| File chunk message | 4 MiB plus 64 KiB |
| Opus packet on a side stream | 4 KiB |
| Direct addresses on a ticket | 8 |

Non-finite floats are rejected. Field number 0 is rejected. A truncated length
is an error. A singular field uses the last value. Repeated fields accept
packed or unpacked encoding.

## Messages implemented

`Hello`, `HelloAck`, `Granted`, `PermissionChanged`, `DisplayInfo`,
`VideoFrameHeader`, `Feedback`, `Close`, `KeyEvent`, `PointerEvent`,
`InputEvent`, `ClipboardText`, `FileChunk`, `Control`, `InviteTicket`,
`DhtAddressRecord`, `SignedDhtRecord`.

Video bytes are not a protobuf field. They follow `VideoFrameHeader` on a
unidirectional stream, described below. `InputEvent` sets exactly one of
`key` or `pointer`.

## Authorization

`InviteTicket` is signed with the signature field cleared. Verification checks
the signature, a maximum lifetime of 15 minutes, at most eight non-empty
addresses, and replay of the signature. `ticket_grants_access()` is constant
false. A `Granted` message is produced by `cbdesk-session` after consent or an
explicit unattended policy. No message asks a peer to hide the session.
`supports_hidden_session()` is constant false.

## Control stream

The first bidirectional stream is the control channel. Each message is a
big-endian `u32` length plus one `Control` protobuf body. The length counts
only that body and must be from 1 to 64 KiB. Exactly one of `hello`,
`hello_ack`, `granted`, or `close` is set. Bytes that are not this frame,
including an echo probe, are `PROTOCOL_VIOLATION`. The probe is not reflected
and it does not grant a session.

`dial_verified_ticket` checks the signature, lifetime, and replay cache, then
dials `direct_addrs` only. Relay URLs on the ticket are ignored. The
connection is then handed to `HostSession`. The ticket is not an argument of
admission.

Interactive approval sends `HelloAck` with `AUTH_INTERACTIVE`, then waits.
The wait is 30 seconds from the moment the prompt is shown
(`APPROVAL_TIMEOUT_MS`). The 3 second first-contact delay sits inside that
window. No answer becomes `Close` with `TIMEOUT_USER_RESPONSE`. Allow is
still refused until the first-contact delay has elapsed. `Granted` is sent
only after `HostSession::approve` or an explicit unattended policy. This
slice's controller offers interactive approval, so the unattended path is not
taken.

## View frames

After `authorize(View)`, the host opens one unidirectional QUIC stream per
frame. The bytes are:

1. big-endian `u32` length of the `VideoFrameHeader`
2. that header
3. big-endian `u32` length of the payload
4. the payload

An empty header or an empty payload is rejected. The two lengths plus both
bodies must be at most 16 MiB. The payload is one VP8 or AV1 picture.
Negotiation selects VP8 when both peers offer it (`Codec::Vp8`) and falls
back to AV1 (`Codec::Av1`). `keyframe` is true for the first full VP8 frame
and for every AV1 still. A later full VP8 frame may clear that flag when it
is an inter frame. `damage_width` and `damage_height`, when both are
non-zero, mean the payload is a keyframe of a crop. The origin is
`damage_x`, `damage_y`. That header still has `keyframe` false. A viewer
places the crop and must not feed those bytes to the full-frame decoder.
Zero damage fields are a full frame, which is also how a header written
before those fields existed decodes. `unchanged` is defined. The host does
not send it: an empty payload is still rejected, and a picture that matches
the previous one is skipped. A unidirectional stream, or an extra
bidirectional stream, opened before the grant is `PROTOCOL_VIOLATION`. The session closes and no
permission is granted. After a frame is sent, the host keeps the connection
up until the controller finishes the control stream or the frame budget is
exhausted and the controller leaves. The controller reads each frame before
it releases the grant. Closing the connection first discards a frame that
is still inside QUIC. `--frames 0` keeps sending until that close. A
positive count stops after that many frames and then waits for the
controller.

The picture is YUV. The live host sends the capture cropped to even
dimensions. The synthetic host sends a gray barcode. A picture that matches
the previous one is not sent again. A dirty region that covers at most a
quarter of the picture, and is smaller than the frame on both axes, is
cropped on a 16-pixel grid and sent as its own VP8 keyframe with the damage
fields set. A larger change stays on the full-frame inter encoder. Feedback
from the viewer can lower or raise the VP8 quantizer on a live screen. The
synthetic barcode stays at quantizer 0.

## Side streams

File chunks, Opus packets, feedback, and two empty acknowledgements share a
unidirectional framing that cannot be mistaken for a length prefix. The
first word is the big-endian magic `0x43424446` (`CBDF`). Then one kind
byte, a big-endian `u32` body length, and the body. An empty body is
allowed.

| Kind | Value | Body | Grant |
| --- | --- | --- | --- |
| File | 1 | one `FileChunk` | `FileWrite` toward the host, `FileRead` toward the viewer |
| Audio | 2 | one Opus packet, at most 4 KiB | `Audio` |
| Feedback | 3 | one `Feedback` | `View` |
| Input acknowledgement | 4 | empty | sent by the host after it accepts one input event |
| File acknowledgement | 5 | empty | sent by the host after one file is renamed into place |

`FileChunk.compressed` must be false. zstd is not linked. `hash` is 32
bytes. `root` is 32 bytes on the last chunk and empty otherwise. The CLI
stores the file under the directory given to `--files-root`. A missing
grant closes the session with `PROTOCOL_VIOLATION`. The viewer waits for
the matching acknowledgement before it drops the connection.

## Input events

An input event is its own unidirectional stream: a big-endian `u32` length
and one `InputEvent`. The host reads it only after `authorize(Input)`.
Without that grant the event is not injected, the host sends `Close` with
`PROTOCOL_VIOLATION`, and the session closes. Key codes are printable ASCII
code points, or the private-use names in `cbdesk-input` (`KEY_ENTER` is
`0xE000` through `KEY_DOWN` at `0xE007`). Modifier bits are shift 1, ctrl 2,
alt 4, meta 8. Pointer buttons are left 1, right 2, middle 4.

## Clipboard text

`ClipboardText` is one bidirectional stream, separate from video and input
so those unidirectional frames stay unchanged. The bytes are a big-endian
`u32` length and one `ClipboardText` protobuf. The text is at most 60 KiB.
Empty text clears the peer's text clipboard. The host accepts the stream
only after `authorize(Clipboard)`. Without that grant the session closes
with `PROTOCOL_VIOLATION` and nothing is written to the operating-system
clipboard. Files, images, and HTML are not this message. The 64 MiB clipboard
ceiling remains the cap for a later reassembly path.

## Errors

Each known `ErrorCode` has `error.<name>` and `error.<name>.hint` keys in both
catalogs. Unknown numeric values use `error.unknown`.

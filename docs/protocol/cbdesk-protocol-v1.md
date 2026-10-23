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
| Clipboard | 64 MiB default and absolute cap |
| Video frame | 16 MiB |
| Direct addresses on a ticket | 8 |

Non-finite floats are rejected. Field number 0 is rejected. A truncated length
is an error. A singular field uses the last value. Repeated fields accept
packed or unpacked encoding.

## Messages implemented

`Hello`, `HelloAck`, `Granted`, `PermissionChanged`, `DisplayInfo`,
`VideoFrameHeader`, `Feedback`, `Close`, `KeyEvent`, `PointerEvent`,
`InputEvent`, `Control`, `InviteTicket`, `DhtAddressRecord`,
`SignedDhtRecord`.

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
bodies must be at most 16 MiB. This slice's payload is one AV1 still
(`Codec::Av1`, `keyframe` true). The capability advertised for that path is
`VideoAv1`. VP8 is not produced. A unidirectional stream, or a second
bidirectional stream, opened before the grant is `PROTOCOL_VIOLATION`. The
session closes and no permission is granted. After the frame is sent, the
host reads the control stream until the controller finishes it. The
controller does that only after it has read the frame. Closing the
connection first discards a frame that is still inside QUIC.

## Input events

An input event is its own unidirectional stream: a big-endian `u32` length
and one `InputEvent`. The host reads it only after `authorize(Input)`.
Without that grant the event is not injected, the host sends `Close` with
`PROTOCOL_VIOLATION`, and the session closes. Key codes are printable ASCII
code points, or the private-use names in `cbdesk-input` (`KEY_ENTER` is
`0xE000` through `KEY_DOWN` at `0xE007`). Modifier bits are shift 1, ctrl 2,
alt 4, meta 8. Pointer buttons are left 1, right 2, middle 4.

## Errors

Each known `ErrorCode` has `error.<name>` and `error.<name>.hint` keys in both
catalogs. Unknown numeric values use `error.unknown`.

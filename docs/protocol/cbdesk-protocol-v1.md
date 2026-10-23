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
`VideoFrameHeader`, `Feedback`, `Close`, `InviteTicket`, `DhtAddressRecord`,
`SignedDhtRecord`.

Video bytes are not carried inside `VideoFrameHeader`. The header is only the
metadata a later media crate will put beside a frame.

## Authorization

`InviteTicket` is signed with the signature field cleared. Verification checks
the signature, a maximum lifetime of 15 minutes, at most eight non-empty
addresses, and replay of the signature. `ticket_grants_access()` is constant
false. A `Granted` message is produced by `cbdesk-session` after consent or an
explicit unattended policy. No message asks a peer to hide the session.
`supports_hidden_session()` is constant false.

## Errors

Each known `ErrorCode` has `error.<name>` and `error.<name>.hint` keys in both
catalogs. Unknown numeric values use `error.unknown`.

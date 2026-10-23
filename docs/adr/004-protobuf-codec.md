<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-004: Protobuf messages, hand-written codec for now

- Status: Accepted
- Date: 2026-10-23

## Context

Session messages need a stable schema, unknown-field tolerance, and hard size
limits. The usual Rust stack is `prost` plus `protoc`.

## Options

- `prost` generated code. Familiar, and unknown fields are easy to drop by
  accident. CI would need `protoc`.
- A hand-written codec that preserves unknown fields and enforces the limits
  in one place. The `.proto` file stays as the readable schema.

## Decision

`proto/cbdesk/v1/session.proto` documents field numbers. `cbdesk-proto` is the
codec CI tests. Unknown field numbers round-trip. Non-finite floats, field 0,
truncated lengths, and oversize control messages are errors. Switching to
`prost` later is acceptable if the unknown-field tests stay red when a field
is dropped. This build speaks major version 1 only. The N / N-1 helper is
tested so a second major version has a written rule.

## Consequences

The codec is more code to maintain than a generator. It does not cover every
protobuf feature (groups are rejected). Media payloads are not in this codec
beyond `VideoFrameHeader`.

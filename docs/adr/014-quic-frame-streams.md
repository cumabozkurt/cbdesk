<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-014: One QUIC stream per video frame

- Status: Accepted
- Date: 2026-10-23

## Context

Head-of-line blocking inside a single reliable stream makes a lost packet
delay later frames. The plan sends each video frame on its own QUIC stream
and uses unreliable delivery where the stack allows it. Control messages stay
reliable and ordered.

## Options

- One reliable stream for the whole session. Easy, and a loss stalls video.
- A datagram frame plus a reliable control stream. Fewer streams, more
  reassembly rules.
- One stream per frame, with `VideoFrameHeader` as the reliable description
  of codec, display, and frame number.

## Decision

The per-frame stream is the target. `VideoFrameHeader` is the metadata this
milestone can encode. No QUIC stack is linked (ADR-006), so no stream is
opened. Reliability of control messages (`Hello`, `Granted`, `Close`) is
separate: those stay ordered.

## Consequences

Implementing video means opening streams, not stuffing frames into the memory
`Conn` as a side effect. The 16 MiB frame cap in `cbdesk-proto` applies to
that payload. The memory transport is a byte pipe for tests and does not model
stream isolation.

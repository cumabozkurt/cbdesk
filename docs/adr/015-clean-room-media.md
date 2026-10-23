<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-015: Keep the clean-room core for capture, codec, and input

- Status: Accepted
- Date: 2026-10-23

## Context

ADR-002 left open a later import of RustDesk for platform code. The master
plan's M0 exit is a fork that builds on three desktops. The protocol, consent
rules, and direct QUIC path in this tree are not that project's stack. A
drop-in fork would replace those checks and would have to be re-audited for
hidden sessions, ticket-as-grant, default relays, and the ports in ADR-009.

## Options

- Import that tree and delete or wrap the parts that disagree with consent,
  the relay policy, and `cbdesk/1`. Attribution is immediate. The media path
  arrives as a second stack beside the one already reviewed.
- Keep this core. Take screen capture, a software codec, and input injection
  from ordinary platform libraries, behind the grants this tree already
  enforces.

## Decision

The second option. This repository does not vendor or fork RustDesk. Capture
uses xcap 0.9.5 (X11, the Wayland Screenshot portal for a still, and the
Wayland ScreenCast plus PipeWire path when `WAYLAND_DISPLAY` is set; Windows
and macOS through the same crate). The first software codec in this tree was
AV1: rav1e with its assembler and git tooling turned off, and rav1d-safe
under AGPL-3.0-only. Input uses enigo, and only after `authorize(Input)`.
Tests record events. They do not move the machine's pointer. The VP8
encoder this tree links now is recorded in the implementation note below.

ADR-002 still records why M0 did not start from an import. This ADR closes
the "import platform code later" option for capture, codec, and input. A
future import would be a new decision, with upstream copyright notices and
`tools/rebrand-check.sh` still in force.

## Consequences

M0's exit criterion ("the fork builds on three desktops") stays unmet on
purpose. CI builds this workspace on Linux, Windows, and macOS instead.
Hardware codecs and a Flutter UI are still absent. The session indicator
still cannot be disabled, and a ticket is still only a locator.

## Implementation note (2026-10-23)

VP8 is oxideav-vp8 0.2.7. The license is MIT. Default features stay off, so
the registry feature and its extra framework are not linked. The crate is
pure Rust. This build does not link a system libvpx. The first frame, and
any frame after a size change, is a keyframe. A later frame can be a
zero-motion inter frame. A unit test checks the inter-frame tag and still
reads the barcode at quantizer 0. Color is YUV. A gray barcode stays gray
enough for that reader. When both peers offer VP8 and AV1, negotiation
selects VP8. T-MED-001 is that both-sides-VP8 selection. T-MED-004 in this
tree is a short barcode stream, including that inter frame. It is not a
60-second soak and it is not a measured 1080p run. The live pacer targets
33 ms.

Audio in this tree is a synthetic Opus tone (rusty-opus), after an Audio
grant. It does not open a microphone. File bytes are a separate crate and
are not a codec.

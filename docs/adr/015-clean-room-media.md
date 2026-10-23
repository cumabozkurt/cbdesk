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
Hardware codecs are still absent. The Flutter shell, motion inter frames,
and device audio are recorded in the implementation note. The session
indicator still cannot be disabled, and a ticket is still only a locator.

## Implementation note (2026-10-23)

VP8 is oxideav-vp8 0.2.7. The license is MIT. Default features stay off, so
the registry feature and its extra framework are not linked. The crate is
pure Rust. This build does not link a system libvpx. The first frame, and
any frame after a size change, is a keyframe. A later frame of the same
size is a motion-compensated inter frame. The encoder calls
`encode_p_frame_multi_ref`, which runs that crate's diamond search and can
emit NEWMV, NEARESTMV, NEARMV, ZEROMV, or SPLITMV. A unit test checks the
inter-frame tag and still reads the barcode at quantizer 0. A second test
shifts a 16-pixel white block by one macroblock at quantizer 96 on a 48×16
picture and checks that the moved columns survive. Color is YUV. When both
peers offer VP8 and AV1, negotiation selects VP8. T-MED-001 is that
both-sides-VP8 selection. T-MED-004 in this tree is the short barcode stream
plus that 48×16 shift. It is not a 60-second soak and it is not a measured
1080p run. The live pacer targets 33 ms. Software encode at a display's
native size can miss that target; the pacer then raises the quantizer.

Audio is rusty-opus after an Audio grant. `--tone` is a synthetic packet
and does not open a device. `--mic` and `--speaker` open the default
devices through cpal 0.15.3. A missing device is an error. This tree does
not record that a person heard the stream. The Flutter shell can open the
default microphone after an Audio grant. A missing device does not end
that session. File bytes are a separate crate and are not a codec. The
Flutter shell shows the decoded frame. On a desktop host that frame is
`capture_picture` when nothing is queued. The 48×16 motion pair remains
the queued test picture. Android frames enter the same queue only after
the system capture dialog, scaled so the longer side is at most 960.
Hardware codecs are still absent.

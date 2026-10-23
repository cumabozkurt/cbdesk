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
and macOS through the same crate). The software codec is AV1: rav1e with its
assembler and git tooling turned off, and rav1d-safe under AGPL-3.0-only.
VP8 is not linked. The crates that encode VP8 require a system libvpx, which
this CI cannot assume on Windows. Input uses enigo, and only after
`authorize(Input)`. Tests record events. They do not move the machine's
pointer.

ADR-002 still records why M0 did not start from an import. This ADR closes
the "import platform code later" option for capture, codec, and input. A
future import would be a new decision, with upstream copyright notices and
`tools/rebrand-check.sh` still in force.

## Consequences

M0's exit criterion ("the fork builds on three desktops") stays unmet on
purpose. CI builds this workspace on Linux, Windows, and macOS instead.
T-MED-001 and T-MED-004 ask for a VP8 stream. This slice speaks
`Capability::VideoAv1` and does not claim those tests. Hardware codecs,
audio, and a windowed UI are still absent. The session indicator still
cannot be disabled, and a ticket is still only a locator.

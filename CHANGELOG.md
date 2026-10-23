# Changelog

This file follows Keep a Changelog. The project uses semantic versioning.
Version 0.0.0 is not a release.

## [Unreleased]

### Added

- Workspace layout and the first crates: protocol codec, identity, discovery
  records, in-memory transport, host session policy, and test doubles.
- Direct QUIC endpoint on iroh 1.4.1 with n0 relays and n0 discovery left
  uninstalled. Rust is pinned to 1.91.0, the `rust-version` of that release.
  T-NET-001 is a loopback echo whose resolver must not be asked for a name.
- A verified invite ticket dials its direct addresses. Relay URLs on the
  ticket are not contacted. The host grants that connection only after
  consent. An unanswered approval closes at 30 seconds with
  `TIMEOUT_USER_RESPONSE`. An echo probe is `PROTOCOL_VIOLATION` and does
  not grant a session.
- LAN announce and browse on the tested mDNS TXT records and the `CBDS`
  broadcast body. Public networks and a disabled user send nothing.
- GitHub Actions workflow for rustfmt, cargo-deny, RustSec audit, clippy, and
  tests on Linux, Windows, and macOS.
- ADRs 001–015, threat-model draft, and contribution documents.
- Turkish and English message catalogs for the errors and consent strings that
  exist so far.
- One unidirectional QUIC stream per video frame, opened only after
  `authorize(View)`. A side stream before the grant is `PROTOCOL_VIOLATION`.
  An input event uses its own unidirectional stream and is accepted only
  after `authorize(Input)`.
- Software AV1 (rav1e, assembler and git tooling off; rav1d-safe under
  AGPL-3.0-only) for one grayscale still. VP8 is not linked.
- Operating-system screen capture through xcap 0.9.5, and keyboard/pointer
  injection through enigo after an Input grant. Tests record input events
  and do not move the pointer.
- `cbdesk host` and `cbdesk view`: after consent, one synthetic AV1 frame
  can be written to a PPM. `--screen` captures the display. The ticket is
  still only a locator.
- ADR-015 keeps the clean-room core. The M0 fork exit stays unmet on purpose.

This tree is not derived from RustDesk. See ADR-002 and ADR-015.

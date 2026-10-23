# Changelog

This file follows Keep a Changelog. The project uses semantic versioning.
Version 0.0.0 is not a release.

## [Unreleased]

### Added

- VP8 inter frames and color. The first frame is a keyframe. A later frame
  can be a zero-motion P-frame. A unit test still reads the barcode, and a
  color sample survives a keyframe and an inter frame. The live path keeps
  the capture's even size. The pacer targets 33 ms and raises the quantizer
  when encode time, loss, or delay says so. This tree has no 1080p timing
  measurement.
- A scripted pointer and Enter travel over QUIC and the host prints them
  after an Input grant. The window uses that mapping. The host sends an
  empty acknowledgement so the last event is still read if the viewer then
  closes.
- Resumable file transfer. Chunks are uncompressed BLAKE3 pieces with a
  bitmap in a sidecar. `file_write` stays high-risk. The CLI `--put` path
  reads at most 32 MiB. zstd is not linked.
- Synthetic Opus audio through rusty-opus after an Audio grant. No
  microphone or speaker device is opened. The library jitter buffer waits
  for 40 ms and drops audio older than 120 ms.
- An operator-configured relay. Forbidden suffixes and n0 defaults still
  fail closed. The default endpoint still has an empty relay list. With a
  relay set, HTTPS net-report probes run only against that relay so a home
  relay can be chosen. A loopback relay test carries one probe with UDP
  off. Symmetric NAT is unmeasured. This project does not run a relay.
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
  AGPL-3.0-only) for one grayscale still.
- Software VP8 through oxideav-vp8 0.2.7 (MIT, default features off, no
  system libvpx). Negotiation prefers VP8 when both peers offer it. AV1 is
  the still-picture fallback.
- Operating-system screen capture through xcap 0.9.5, and keyboard/pointer
  injection through enigo after an Input grant. Tests record input events
  and do not move the pointer.
- `cbdesk host` streams frames until the viewer closes (`--frames 0`).
  `cbdesk view --window` shows them with winit and softbuffer and forwards
  the pointer and keyboard only after an Input grant. `--ppm` still writes
  the first frame. `--screen` captures the display. The ticket is still
  only a locator.
- Plain-text clipboard sync on a bidirectional stream after a Clipboard
  grant, capped at 60 KiB. Files and images are not this message.
- The device seed is loaded from the OS keychain (Secret Service, Keychain,
  or Credential Manager). A store that does not answer yields an in-memory
  identity for that process. The seed is not written to a file.
- LAN announce and browse can join `224.0.0.251:5353` when that port can be
  shared. A busy 5353 keeps sending to the group and listens on an ephemeral
  port. `--lan` is the operator opting into a private profile.
- ADR-015 keeps the clean-room core. The M0 fork exit stays unmet on purpose.

This tree is not derived from RustDesk. See ADR-002 and ADR-015.

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
- ADRs 001–014, threat-model draft, and contribution documents.
- Turkish and English message catalogs for the errors and consent strings that
  exist so far.

This tree is not derived from RustDesk. See ADR-002.

<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-006: iroh 1.4.1 on Rust 1.91, with no n0 defaults

- Status: Accepted
- Date: 2026-10-23

## Context

The plan uses iroh for QUIC and names `Endpoint::empty_builder` so no n0
relay or discovery server is installed. T-NET-001 requires that a session
does not contact `*.iroh.link`, `n0.computer`, `n0.io`, `iroh.network`, or a
CBDesk central domain. This project has no central domain.

The first milestone pinned Rust 1.83 and did not depend on iroh, because
every current iroh 1.x release declares `rust-version = "1.91"`. iroh 1.4.1
(2026-10-21) is the current crates.io release. Older 1.x releases declare the
same compiler, so a pin below 1.91 cannot build any of them. Rust 1.91.0 is
therefore the smallest toolchain this dependency accepts. The pin is not
"latest stable".

`Endpoint::empty_builder` does not exist in iroh 1.4.1. `Builder::empty`
starts with no address lookup and no relay transport. `presets::Minimal`
is that empty builder plus the rustls provider selected by the `tls-ring`
feature. `presets::N0` is the opposite: it adds the n0 DNS publisher,
pkarr resolver, and the default relay map, including names under
`iroh.link`.

## Decision

- `rust-toolchain.toml` and the workspace `rust-version` are 1.91.0.
- `cbdesk-net` depends on iroh 1.4.1 with default features off and
  `tls-ring` on. The port-mapper feature stays off.
- `QuicEndpoint::bind` calls `Endpoint::builder(presets::Minimal)`, then
  `clear_address_lookup` and `PortmapperConfig::Disabled`. An empty relay
  list selects `RelayMode::Disabled` and `NetReportConfig::minimal`, so
  those probes stay off. A non-empty list that has passed the suffix check
  selects `RelayMode::Custom` for those URLs only.
- The endpoint's DNS resolver is an in-process recorder that returns
  NXDOMAIN. The system resolver and the public fallback resolvers are not
  installed.
- `use_n0_defaults` still fails before a socket exists. A relay hint on a
  forbidden suffix still fails before a socket exists. A URL that passes
  that check can be installed. `RelayMode::Default` and `RelayMode::Staging`
  are still not selected. A relay that appears only on a ticket is not
  installed.
- The 32-byte seed passed to bind is the Ed25519 seed. The iroh endpoint id
  is that public key. A test checks the bytes match `DeviceId`.
- T-NET-001 in CI is a loopback echo of those two endpoints. The test fails
  if either resolver recorded any DNS name, and it fails if a recorded name
  matches a forbidden suffix. It also checks that the endpoint advertises no
  relay URL. It does not sniff the runner's other traffic (the runner still
  talks to GitHub). There is still no CBDesk central domain in the binary.

The in-memory `Transport` remains for tests that do not need a socket.
`QuicEndpoint` is a separate type so iroh's connection does not have to
become the memory `Conn`, and so `forbid(unsafe_code)` still holds.

## Consequences

LAN mDNS is a separate crate (`cbdesk-discovery`) and does not share iroh's
sockets. Raising Rust again waits for a newer iroh `rust-version`, and that
bump is another edit of this ADR. `ring`, pulled in by `tls-ring`, needs a
C toolchain on the three desktop CI runners. BLAKE3 in our crates stays on
the `pure` feature.

## Implementation note (2026-10-23)

iroh chooses a home relay from the HTTPS probe in the net report. With
probes left off, a configured relay never becomes connected, so a dial that
has no direct address waits until its timeout. When the operator list is
non-empty, HTTPS probes are on and the captive-portal check stays off
(that check uses port 80 on the relay host). Probes go to the configured
relays. The default empty list still uses `NetReportConfig::minimal`.

A literal loopback relay presents a certificate this process did not obtain
from a public authority. Only then does bind install a verifier that
accepts that certificate. Other relay hosts keep normal verification.
`presets::N0` stays unused.

The loopback proof starts iroh's test relay helper in the test process,
binds two endpoints with UDP transports cleared, and echoes one probe.
That helper is a dev-dependency. The CLI does not spawn it. `cbdesk net-check`
reports local UDP binds, CGNAT address space, and an optional loopback path.
A live symmetric NAT is still unmeasured. The mapping classifier is a
decision rule in tests. T-NET-001 is still the direct loopback echo with an
empty relay list and zero DNS queries.

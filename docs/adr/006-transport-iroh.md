<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-006: iroh is the target transport, not a dependency yet

- Status: Accepted
- Date: 2026-10-23

## Context

The plan uses iroh for QUIC, with `Endpoint::empty_builder` so no n0 default
relay or discovery server is installed. T-NET-001 captures traffic and checks
that those names are never queried. The Rust in this environment is 1.83.
Current iroh releases expect a newer compiler. Pulling iroh in this pass would
either fail CI or force an unreviewed toolchain jump.

## Options

- Add iroh now and raise the toolchain in the same change.
- Define `Transport`, ship an in-memory implementation, and enforce the relay
  policy on configuration. Record the packet-capture test as not done.

## Decision

The second option. `cbdesk-net` does not depend on iroh. `Transport` is a
native async trait and returns the concrete `Conn` used by the memory hub,
not a trait object. That keeps `forbid(unsafe_code)` (the `async-trait` crate
expands to unsafe). A later QUIC adapter can wrap its connection or replace
the hub behind the same method names.

T-NET-001 in this milestone is the configuration half: the default
`EndpointConfig` has an empty relay list, `use_n0_defaults` must be false, and
a dial whose hints match `iroh.link`, `n0.computer`, `n0.io`, or
`iroh.network` on a DNS label boundary fails before a connection is offered.
`notiroh.link` and `evil-iroh.link` do not match. This does not capture
packets. The full T-NET-001 remains open.

## Consequences

There is no NAT traversal, no QUIC, and no real concurrency test of the
transport. The path state machine and the reconnect ladder are pure functions
with unit tests. Port 22116 is reserved for the future QUIC listener (ADR-009).

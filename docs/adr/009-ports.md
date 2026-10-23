<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-009: Default UDP ports

- Status: Accepted
- Date: 2026-10-23

## Context

Another remote-desktop program uses UDP and TCP ports 21115 through 21119.
CBDesk needs to run beside that program on one machine. The master plan leaves
the final numbers to this ADR.

## Options

- Reuse 21115–21119. Immediate conflict.
- Pick a nearby unused pair and document it.

## Decision

- QUIC listen port: 22116/udp (`QUIC_PORT`).
- LAN broadcast fallback: 22118/udp (`LAN_BROADCAST_PORT`).

mDNS service type: `_cbdesk._udp.local.` The mDNS socket uses the standard
port 5353/udp and group `224.0.0.251`. That port is not a CBDesk allocation.
No TCP rendezvous port is allocated.

`QuicEndpoint` binds whatever address the caller passes. Tests use an
ephemeral loopback port. A fixed listener uses 22116/udp. LAN announce and
browse send the tested mDNS and `CBDS` encodings; tests aim those datagrams
at ephemeral loopback sockets because 5353 is often already taken. Calling
`LanSocketPlan::ipv4_defaults` selects the standard group and 22118.

## Consequences

Firewall docs and installers must use these numbers. Changing them is an
incompatible discovery change and needs a new ADR.

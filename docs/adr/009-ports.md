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

mDNS service type: `_cbdesk._udp.local.` No TCP rendezvous port is allocated.
Nothing listens on these ports yet. The constants exist so later sockets and
the rebrand tests agree.

## Consequences

Firewall docs and installers must use these numbers. Changing them is an
incompatible discovery change and needs a new ADR.

<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-010: Service, agent, and netd are separate processes

- Status: Accepted
- Date: 2026-10-23

## Context

The plan splits privileged host work (service), the per-session UI agent, and
the network process (`netd`). The split limits what a network bug can do and
lets the agent run in the user's session.

## Options

- One process for the whole host. Simpler, and a memory-safety bug in the
  parser is a bug in the privileged process.
- Three processes with a small local IPC, as in the plan.

## Decision

The split is the target. `cbdesk-service`, `cbdesk-agent`, and `cbdesk-netd`
are README placeholders. They are not workspace members and they must not gain
a hidden mode when they are implemented. The in-memory `Transport` is the
stand-in for netd's byte path in tests.

## Consequences

IPC, service install, and session-0 isolation are not designed in detail here.
Implementing any of the three binaries starts a new milestone, not a quiet
addition to `cbdesk-net`.

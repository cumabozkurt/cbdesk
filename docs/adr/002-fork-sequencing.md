<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-002: Start from the modular crates, import RustDesk later

- Status: Accepted
- Date: 2026-10-23

## Context

The master plan's M0 describes forking RustDesk, preserving that git history
for attribution, and rebranding it before new protocol work. This repository
was created with a README and the AGPL license only. It has no upstream
history to preserve.

## Options

- Import that tree now, then delete or wrap the pieces that do not match the
  crate layout. Attribution is immediate. The first review is a large rebrand.
- Write the destination crates first (this pass), and import AGPL-covered
  platform code later as a separate change that keeps upstream copyright
  notices.

## Decision

This repository follows the second option. The crates in this pass are not
derived from RustDesk. A later change may vendor or port platform code from
that project. That change must:

- keep the original copyright and license notices,
- pass `tools/rebrand-check.sh`, which allows the upstream name only in
  attribution files,
- avoid that project's default ports (see ADR-009) and its server hostnames.

Until that change lands, READMEs and `NOTICE` must not claim a fork.

## Consequences

M0's exit criterion ("the fork builds on three desktops") is not met.
CI builds this workspace instead. Screen capture and input will lag the
protocol work. The benefit is that identity, consent, and the relay policy
are reviewable without a large unrelated diff.

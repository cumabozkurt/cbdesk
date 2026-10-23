<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-012: Synchronizing "my devices"

- Status: Proposed
- Date: 2026-10-23

## Context

Paired devices should share an address book without a project server. The plan
leans toward a CRDT. Automerge is the library mentioned as a candidate, not as
an accepted dependency.

## Options

- A central account. Rejected by the product principles.
- Last-write-wins files copied by hand.
- A CRDT (Automerge or another) over the encrypted transport, once that
  transport exists.

## Decision

No library is accepted. `cbdesk-sync` is a placeholder. A future ADR has to
name the CRDT, the threat model for a malicious peer in the group, and how
deletion of a trusted device propagates. This record only stops the project
from growing a silent default account.

## Consequences

There is no multi-device address book. Pairing state, when it exists, will be
local until this ADR is replaced by an Accepted one.

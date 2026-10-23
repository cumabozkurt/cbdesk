<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-008: Short-code password exchange

- Status: Proposed
- Date: 2026-10-23

## Context

A short code is `NNN-WORD-WORD`. The slot is public. The two words are a
low-entropy password (22 bits plus the rate limit). The plan uses a PAKE so
the network does not see the words and a mediator cannot test the password
offline.

## Options

- Send the words as a hash. Offline guessing is immediate.
- SPAKE2 with the words as the password and the transcript bound to both
  device keys.
- OPAQUE for this path as well. Heavier, and a better fit for the long-term
  password than for a 10 minute code.

## Decision

No protocol bytes are defined in this milestone. The candidate is SPAKE2, with
the transcript bound to the two Ed25519 keys and to the slot epoch. OPAQUE
stays the candidate for the permanent password (ADR-005) and is not reused
here without a new decision.

`cbdesk-discovery` may generate and parse codes. It must not put the words
into a ticket or a DHT record. Edit-distance-1 collisions in the word lists
are a known gap and are not a reason to invent a PAKE in the same change.

## Consequences

A short code does not authenticate anyone yet. The host policy still has to
approve the session after a future PAKE succeeds.

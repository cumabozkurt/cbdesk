<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-008: Short-code password exchange

- Status: Accepted
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

The desk host and the desk controller run SPAKE2 with the published `spake2`
0.4.0 crate (`Ed25519Group`). The password is the two lowercase words joined
by `-`. The public slot is not part of the password. The transcript is the
slot as a big-endian `u16`, then the host Ed25519 key, then the controller
Ed25519 key. Identities are `cbdesk-host` and `cbdesk-controller` prefixed
to that transcript. Each side confirms with a BLAKE3 keyed hash over
`cbdesk-spake2-confirm-v1`, the transcript, and the role. The SPAKE2 output
and the MAC key are wiped. `spake2`'s `finish` succeeds even when the
passwords differ, so a mismatch is the confirmation tag, not that call.

The exchange is the first bidirectional stream. Its first word is the magic
`CBPK` (`0x4342504B`), which cannot be a control length because a control
frame is at most 64 KiB. A desk host that sees anything else closes the
stream and does not arm consent. The CLI host does not run this exchange, so
a controller that opens control first still reaches the CLI consent path.

The words are not written into an invite ticket or a DHT record. A short
code alone does not locate a computer. After the tags match, the host still
runs the consent policy, including the first-contact delay. A wrong code
does not open that prompt.

OPAQUE stays the candidate for the permanent password (ADR-005) and is not
implemented.

## Consequences

`short_code_authenticates` is true: the words confirm pairing, and the
ticket is still only a locator. The construction has not had an external
review. Edit-distance-1 collisions in the word lists are still a known gap.
No production path hashes the words with raw SHA-2 or compares the tags with
`==`.

<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-005: Use maintained primitives, do not invent one

- Status: Accepted
- Date: 2026-10-23

## Context

Device identity, key agreement, backup key stretching, and fingerprints need
cryptography. The master plan also names OPAQUE for the long-term password and
a PAKE for short codes.

## Options

- A project-specific construction. Faster to sketch, unfit to ship.
- `ed25519-dalek`, `x25519-dalek`, BLAKE3, HKDF-SHA256, and Argon2id, with
  OPAQUE and the short-code PAKE left as selected designs rather than code.

## Decision

Accepted for the primitives that are called:

- Ed25519 device keys (`ed25519-dalek` 2.1). The device id is the public key.
- X25519, rejecting the all-zero shared secret.
- BLAKE3 for the short fingerprint and for ticket-replay digests.
- HKDF-SHA256 for the DHT epoch seed. That seed is not public-key blinding
  (ADR-007).
- Argon2id version 0x13 for a future identity-backup key. Parameters: 64 MiB,
  3 passes, parallelism 1.

OPAQUE is the selected design for a permanent password and is not integrated.
Short-code PAKE is ADR-008 and is Proposed. No production path may hash a
password with a raw SHA-2 or compare secrets with `==` on a signature.

## Consequences

Versions are pinned because newer releases need Rust 1.85. `SigningKey` in
ed25519-dalek 2.1 zeroizes on drop and does not implement the `Zeroize` trait.
`Identity` wipes by replacing the key with a zero seed so the old seed's drop
runs. RFC 8032, RFC 7748, RFC 5869, and an Argon2id vector are tests.

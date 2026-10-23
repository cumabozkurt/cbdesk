<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-007: DHT public-key blinding is not shipped

- Status: Proposed
- Date: 2026-10-23

## Context

The plan publishes DHT address records under a key blinded the way Tor v3
blinds an onion service key, so a record is not linkable to the long-term
device id. A sketch that only HKDF-expands the long-term seed cannot be checked
by a peer who knows only the public key. Shipping that sketch as "blinding"
would be a false claim.

## Options

- Publish under the long-term key and say so.
- Publish under the HKDF epoch seed and call it blinding.
- Specify a reviewed blinding construction, then implement it.

## Decision

Blinded publishing returns `BlindingPendingReview`. Callers can sign with the
long-term key only. `dht_epoch_seed` remains, is tested, and is documented as
not the blinding construction. Records that set `publish_ip` false and still
contain a direct address are rejected.

This ADR stays Proposed until a cryptographer has written the actual blinding
inputs, the epoch rotation, and how a peer recomputes the blinded key from
public information.

## Consequences

DHT records in this tree are linkable to the device id. Privacy documentation
must keep saying that. A later implementation has to fail closed the same way
until the tests for unlinkability exist.

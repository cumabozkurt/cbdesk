<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-013: Updates use TUF

- Status: Accepted
- Date: 2026-10-23

## Context

A later updater must resist rollback, freeze, and a single stolen maintainer
key. The plan names TUF.

## Options

- HTTPS plus a single signature. Simple, and one stolen key ships malware.
- TUF with a threshold on the targets or root role.

## Decision

TUF is the selected design. It is not implemented. No update URL is contacted.
No root keys are generated in the repository, because a key checked into git
would be a placeholder people might trust. `cbdesk-update` stays a README
until a threshold, a root ceremony, and an offline root are written down.

## Consequences

There is no auto-update. Users of a future build will need a documented manual
upgrade path before TUF is ready. Claiming "secure updates" in a release note
before this crate exists would be false.

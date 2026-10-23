<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-001: AGPL-3.0-or-later and the DCO

- Status: Accepted
- Date: 2026-10-23

## Context

The master plan requires a strong copyleft license so a network service that
uses the program shares its changes, and it rejects a contributor license
agreement so a later relicensing needs every copyright holder.

## Options

- MIT or Apache-2.0. Easy to embed, and a hosted build can stay closed.
- GPL-3.0-only. Copyleft for distribution, weaker for a pure network service.
- AGPL-3.0-or-later plus the Developer Certificate of Origin.

## Decision

The repository license is AGPL-3.0-or-later. Contributions use `Signed-off-by`
(DCO 1.1). There is no CLA. `tools/check-dco.sh` fails the CI job when a
commit on a pull request lacks the trailer.

## Consequences

Network use of a modified CBDesk can trigger the AGPL source obligation.
Proprietary modules cannot be linked in and kept closed. The "or later"
clause leaves a door the FSF's future license could use; that was chosen
explicitly. Changing the license later still requires the copyright holders.

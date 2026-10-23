<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-011: Organization certificates

- Status: Proposed
- Date: 2026-10-23

## Context

The plan lets an organization issue certificates that map a device to a role.
Revocation and a monotonic policy version are part of that design. None of the
certificate machinery exists.

## Options

- A custom certificate format.
- Ordinary X.509 with a constrained profile, or a smaller purpose-built
  certificate once the role fields are known.

## Decision

No format is selected. `AuthMethod::OrgCert` is a protocol value so peers can
name it. The session policy treats it as one of the methods that may complete
an unattended grant only when unattended access is explicitly enabled. The
grant is still the safe permission set. There is no revocation list and no
parser.

This ADR stays Proposed until the certificate bytes, the revocation channel,
and the policy-version rule are written down.

## Consequences

Shipping a parser before this ADR is Accepted would freeze a format by
accident. Do not add one.

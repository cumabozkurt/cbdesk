# Governance

CBDesk starts with a founding maintainer and a public ADR record. After 1.0
the project intends to move to a steering committee of three to five people,
as described in the master plan. That committee does not exist yet.

## Decisions

- Product and architecture decisions that close an option are written as ADRs
  in `docs/adr/`.
- ADRs are Accepted, Proposed, or Superseded. A Proposed ADR is not permission
  to ship the design.
- License changes are not made by a contributor license agreement. Contributions
  use the Developer Certificate of Origin (`Signed-off-by`).

## Review

`CODEOWNERS` lists `@cumabozkurt` so the file resolves to the repository owner.
The master plan asks for two reviewers on `cbdesk-crypto`, `cbdesk-net`,
`cbdesk-update`, and release signing. A second reviewer will be required in
branch protection when a second maintainer exists. Until then, crypto and
network changes still need an ADR when they change a security property, and
the pull request must say what was not reviewed by a second person.

## Conduct

`CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1. The interim contact is
the address in that file. It is a personal contact until the project has a
shared security or conduct mailbox.

## What maintainers will not merge

- Hidden, stealth, or silent remote-control modes.
- A default that phones home to a vendor relay, discovery server, or telemetry
  endpoint.
- A claim that a locator (ticket, short code, DHT record, LAN announcement) is
  an authorization.

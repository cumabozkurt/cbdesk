# Security policy

CBDesk is not a finished remote-desktop product. Reports are still welcome for
the code that is already in the repository: identity handling, the protocol
codec, ticket verification, the consent policy, and the relay-host policy.

## Reporting

Use GitHub private vulnerability reporting on this repository:

https://github.com/cumabozkurt/cbdesk/security

There is no project PGP key yet. Do not send a secret, a live device seed, or
an exploit that acts on someone else's computer to a public issue, pull
request, or chat.

Please include the affected revision, the crate, and a way to reproduce the
problem against test keys. Give the project a chance to fix it before publishing
details.

## Response targets

- First response within 72 hours.
- Coordinated disclosure window of 90 days from the first response, unless a
  shorter window is required because the issue is already public or actively
  exploited.

These are targets, not a service agreement. The project is maintained by
volunteers.

## What will not be accepted

- A hidden session, a stealth flag, or a way to turn off the session indicator.
- A change that treats an invite ticket, a short code, or a DHT record as
  permission to view or control a computer.
- A default configuration that contacts a vendor relay or discovery server.

## Supported versions

No release has been published. Only the default branch is in scope.

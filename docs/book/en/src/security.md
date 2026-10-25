# Security

This is a summary of the model the code follows. It is not an external
audit. The longer draft is
[docs/threat-model.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/threat-model.md).

## The person at the host decides

A session does not start because someone has a ticket, a short code, or a
LAN announcement. Those help the other person find this computer. The host
still approves, and the approval lists the permissions. The first contact
waits before Accept can be used. The session indicator cannot be disabled.
There is no stealth setting.

High-risk permissions, including file write, a terminal, blocking local
input, and a privacy cover, are separate from seeing the screen. Quick
support cannot turn on unattended access.

## The network path

Sessions that use the iroh path are QUIC. Payloads on that path are
encrypted. A relay the operator configures can see addresses and how much
data moves. It is not trusted with the password or the session contents.
This project does not run a relay, and known public relay suffixes are
refused by default. A loopback connection on one machine is not evidence
that a NAT hole punch works. A live symmetric NAT has not been measured.

## Passwords and identity

The device id is an Ed25519 public key. The seed stays on the device. A
permanent password library uses `opaque-ke` 4.0.1. The command-line host
does not enroll a password. TOTP exists as a library and is not attached to
the approval prompt. Neither that password composition nor the short-code
exchange has had an external review.

DHT records that are published are linkable to the device id. Publishing
under a blinded key fails closed. That design is not accepted yet.

## Reporting a problem

Use GitHub private vulnerability reporting:

<https://github.com/cumabozkurt/cbdesk/security>

Do not post a device seed or an attack against someone else's computer in a
public issue. An unsigned GitHub prerelease can exist after a `v*` tag.
It is not a supported release. Only the default branch is in scope.
Response targets are in
[SECURITY.md](https://github.com/cumabozkurt/cbdesk/blob/main/SECURITY.md).
They are volunteer targets, not a service agreement.

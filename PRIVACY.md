# Privacy

CBDesk is designed so the project operator does not sit in the path of a
session. This repository does not ship a client that connects anywhere yet.
The rules below are the product rules the code already follows, and the rules
later crates must keep.

## Data the project does not collect

There is no account, no telemetry, and no analytics endpoint in this tree.
Device names are not broadcast unless a later settings screen records an
explicit opt-in. The LAN packet in `cbdesk-discovery` omits the name by default
and uses "CBDesk device" only as the generic label when a name is allowed.

## Data that stays on the devices

- The Ed25519 seed is a secret. `Identity`'s debug output prints the device id,
  not the seed. `Redacted<T>` debug-formats as `[REDACTED]`.
- A short code's words are a password. They are not written into a DHT record.
- An invite ticket carries addresses and a signature. It does not carry a
  permission grant.

## What a relay would see later

A relay, once one exists, may see that two network addresses are exchanging
packets and how much data moves. It must not see screen contents, input, file
bytes, or clipboard contents. That guarantee is not implemented yet, because
there is no encrypted session transport in this milestone. The endpoint policy
rejects the known public relay suffixes so a future stack cannot turn them on
by default.

## DHT

Blinded DHT keys are not implemented. Publishing under a blinded key returns
an error. A record signed with the long-term key is linkable to the device id.
Do not treat the current DHT helper as the unlinkable design in the master plan.

## LAN

Announcements are suppressed when the operating system reports a public
network. Discovery is not authentication. A peer found on the LAN still goes
through the session policy.

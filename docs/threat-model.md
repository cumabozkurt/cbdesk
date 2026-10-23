# Threat model v0.1

This is a first draft for the code that exists. It is not an external review
and it is not a model of screen capture, because capture does not exist.

## Assets

- The Ed25519 seed. Loss of the seed is impersonation of the device.
- Session contents, once a real transport exists: screen, input, files,
  clipboard, audio.
- The person's decision at the host: which permissions were granted, and
  whether a session is in progress.

## Adversaries

- A network attacker who can read, drop, or replay packets.
- A peer who sends unexpected protocol bytes, reused tickets, or a request
  for high-risk permissions on first contact.
- A person who calls the host and tries to talk them through an install.
  This is the fraud case. The product answer is a warning and a consent step,
  not a technical proof.
- A future relay operator. They are honest-but-curious about metadata and
  must not be trusted with plaintext.

## Controls in this tree

- Device ids are Ed25519 public keys. Signatures use the dalek implementation.
  No new primitive is defined here.
- Tickets expire within 15 minutes, bind to the signing key, and are remembered
  by signature so a replay fails. Expired tickets are not stored in the replay
  cache.
- The host session does not open media before a grant. The indicator is
  visible while consent is pending and while the session is active, and the
  disable function is a constant false.
- Unattended grants are a fixed safe set. High-risk permissions are not in
  that set. First contact must opt in before file write, terminal, block
  input, or privacy mode can be included in an interactive grant.
- Quick Support cannot be combined with unattended access. A settings file
  with an unknown `stealth` key fails to parse.
- Authentication failures lock the device id and the source prefix. Consent
  prompts are limited to one per minute, then silenced after three rejects.
- Relay configuration rejects the known public-relay suffixes and rejects
  `use_n0_defaults`. The default list is empty.
- Blinded DHT publish fails closed. The HKDF epoch seed is derived from the
  long-term secret, so it is not the public blinding construction.

## Gaps

- No encrypted transport, so there is no proof of end-to-end confidentiality
  on the wire yet.
- No packet capture test that a socket stays silent.
- No second factor implementation. The session state machine can pause for a
  TOTP step; it does not check a TOTP code.
- No PAKE and no OPAQUE. Short codes are generated and parsed only.
- Rate limits are in memory. A process restart clears them.
- Word lists still contain edit-distance-1 pairs.
- Social engineering is not solved by the 3 second first-contact delay. The
  delay only stops a mis-click. The fraud warning has to be shown by a UI
  that does not exist yet.

## Out of scope for v0.1

Kernel drivers, virtual displays, clipboard malware, supply-chain compromise
of a future updater, and physical access to an unlocked host.

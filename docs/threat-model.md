# Threat model v0.2

This is a draft written with the code. It is not an external review. ADR-007
(DHT blinding), ADR-008 (the SPAKE2 transcript), and the OPAQUE composition
have not had that review.

## Assets

- The Ed25519 seed. Loss of the seed is impersonation of the device.
- The OPAQUE `HostSetup`. It is the server's OPRF state, not the password,
  and losing it lets a new password file be minted.
- The OPAQUE `PasswordRecord`. It is about as sensitive as a password hash.
  The password itself is not stored.
- Session contents after a grant: screen, input, files, clipboard, audio.
- The person's decision at the host: which permissions were granted, and
  whether a session is in progress. The indicator cannot be turned off.

## Adversaries

- A network attacker who can read, drop, or replay packets.
- A peer who sends unexpected protocol bytes, reused tickets, a wrong
  short code, or a wrong password.
- A person who calls the host and tries to talk them through an install.
  The product answer is a warning and a consent step, not a technical proof.
- A relay operator. They can see addresses and traffic volume. QUIC payloads
  are encrypted by iroh. A relay is not trusted with the password.
- Someone who later reads the host's password record. Argon2id is the
  stretching function. It does not make a weak password safe; the policy
  refuses passwords shorter than 10 characters or with a zxcvbn score below 3.

## Controls in this tree

- Device ids are Ed25519 public keys (`ed25519-dalek`). No new signature
  primitive is defined here.
- Tickets expire within 15 minutes, bind to the signing key, and are
  remembered by signature so a replay fails.
- Short-code pairing uses `spake2` 0.4.0. The words are the password. A
  wrong code does not open the consent prompt. The transcript has not had
  an external review.
- A permanent password uses `opaque-ke` 4.0.1 (RFC 9807) with Ristretto255
  and Argon2id (19 MiB, 2 passes, 1 lane; the `argon2` crate defaults).
  Host functions do not take the password. A test checks that the password
  bytes are absent from the registration and login messages. The CLI does
  not enroll a password, so this control is a library. The composition has
  not had an external review.
- TOTP is HMAC-SHA1 as in RFC 6238, checked against that RFC's SHA-1
  vectors. It is not wired to the consent prompt.
- QUIC uses iroh 1.4.1 with `presets::Minimal`, an empty relay list by
  default, and no n0 discovery. Isolated mode rejects a relay and refuses
  DHT publish.
- The host session does not open media before a grant. The indicator stays
  on. Privacy mode, block-input, and recording do not change the operating
  system in this build. Recording is refused unless the host was told.
  Terminal and tunnel gates do not spawn a shell or bind a port.
- Unattended grants are a fixed safe set. High-risk permissions are not in
  that set. Quick Support cannot be combined with unattended access.
- Authentication failures lock the device id and the source prefix.
- DHT records are signed with the long-term device id. That signature is
  linkable to the device. Blinded publish returns `BlindingPendingReview`.
  The HKDF epoch seed is not the blinding construction.
- An access log entry is chained with BLAKE3. The log is in memory. It is
  not a signed transparency log.

## Gaps

- No live symmetric-NAT measurement and no hole-punch success rate.
  `cbdesk net-check` classifies local addresses and can show a loopback
  path. UPnP and DHT reachability are not probed.
- DHT records are linkable to the device id until ADR-007 is accepted.
- The SPAKE2 transcript and the OPAQUE composition have no external review.
- The password CLI is not wired. A caller who types a password still holds
  it in their own buffer. Local enrollment, if added later, would see the
  password in that process for the duration of registration.
- TOTP is not required by the session state machine.
- Rate limits and the access log are in memory. A process restart clears
  them.
- `opaque-ke` 4.0.1 depends on `crypto-bigint` 0.5.5 through
  `elliptic-curve` 0.13.8. RUSTSEC-2026-0387 is a timing issue fixed in
  `crypto-bigint` >= 0.7.6. That line does not fit `elliptic-curve` 0.13,
  and no stable `opaque-ke` uses 0.14. `deny.toml` and `.cargo/audit.toml`
  ignore this one advisory until a stable `opaque-ke` newer than 4.0.1
  no longer depends on `crypto-bigint` 0.5.5. Remove both ignores in that
  bump, and not before `cargo tree -i crypto-bigint -p cbdesk-crypto`
  drops 0.5.5. The arithmetic is not patched here.
- Linux X11, when `cbdesk-hostops` is called with a permit, can grab the
  keyboard and pointer and map a black override-redirect window on the
  display the caller names. Tests use a private Xvfb. Wayland is not
  covered. Windows and macOS return unavailable. The live `cbdesk host`
  session does not call this, so a granted session does not grab input
  by itself.
- A recording notice writes a text file and may run `notify-send`. It is
  not an encrypted recording.
- The terminal is a Unix pseudoterminal running `sh`, or `cmd.exe` pipes
  on Windows. ConPTY is not linked. The tunnel listens on `127.0.0.1` and
  refuses any other destination. Neither is attached to the live session.
- A Linux virtual display is an `Xvfb` process started by
  `open_virtual_display`. It is not a Windows indirect display driver.
  macOS has no virtual-display entitlement here. Live capture does not
  start Xvfb.
- No organization certificate parser (ADR-011) and no CRDT address book
  (ADR-012). No updater and no TUF root (ADR-013).
- Word lists still contain edit-distance-1 pairs.
- Social engineering is not solved by the 3 second first-contact delay.
- Hardware VP8 can be probed through ffmpeg when `hw-vaapi`,
  `hw-mediafoundation`, `hw-videotoolbox`, or `hw-mediacodec` is enabled.
  Those features are off by default. There is no in-process VA-API, Media
  Foundation, VideoToolbox, or MediaCodec context. NVENC, AMF, and Quick
  Sync stay unlinked. A measured 1080p software VP8 frame is not a 30
  frame-per-second result. See `docs/dev/media-measurement.md`.
- Wayland ScreenCast is not executed in CI. See
  `docs/dev/wayland-screencast.md`.

## Out of scope for v0.2

Kernel drivers, a Windows indirect display, clipboard malware, supply-chain
compromise of a future updater, and physical access to an unlocked host.
An Xvfb process in this tree is not a kernel display driver.

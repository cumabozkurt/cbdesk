# M0 status

This note is the status of the master plan. The plan text in
`docs/MASTER_PLAN.md` is unchanged, including its checkboxes.

The repository is not a RustDesk fork. ADR-002 records why the empty tree
starts at the modular crate layout instead of importing that history.
ADR-015 keeps capture, the codec, and input in this clean-room core and
closes that later-import option for those three pieces. Nothing in the Rust
sources is derived from that project.

## Done in this pass

- Master plan stored at `docs/MASTER_PLAN.md`.
- Cargo workspace, Rust 1.91 pin (ADR-006), rustfmt, clippy, and `cargo-deny` policy.
- GitHub Actions: rustfmt, cargo-deny, RustSec audit, DCO, catalog check,
  rebrand scan, and clippy plus tests on `ubuntu-24.04`, `windows-2022`, and
  `macos-14`.
- Contribution documents: `CONTRIBUTING.md`, `GOVERNANCE.md`, `SECURITY.md`,
  `PRIVACY.md`, `TRADEMARK.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `NOTICE`.
- ADRs 001–015. ADR-015 declines a fork for capture, codec, and input.
- Threat model v0.1 and a protocol note for the messages that exist.
- Turkish and English catalogs for consent, the fraud warning, and every
  `ErrorCode` string.
- `cbdesk-proto`: hand-written protobuf codec for `cbdesk/1`, unknown fields
  preserved, size limits, no hidden-session flag.
- `cbdesk-crypto`: Ed25519 device ids, z-base-32, short fingerprint, X25519,
  HKDF, Argon2id backup parameters. RFC test vectors are locked in tests.
- `cbdesk-discovery`: invite tickets (15 minute cap, replay cache, no grant),
  short codes, LAN packet and mDNS TXT shape, runtime announce/browse on those
  encodings, DHT records. Blinded publishing returns an error. Public networks
  do not send.
- `cbdesk-net`: `Transport` trait, in-memory connections, candidate ranking,
  reconnect backoff, path state machine, relay suffix policy, direct QUIC,
  ticket dial, an optional operator relay, the consent handshake, and
  grant-gated view, input, clipboard, file, and audio streams.
- `cbdesk-session`: consent delay, 30 second approval timeout, unattended
  rules, quick-support ban, high-risk opt-in, indicator that cannot be
  disabled, auth rate limits.
- `cbdesk-capture`: synthetic barcode frames, and an operating-system
  capturer (X11, Wayland ScreenCast plus PipeWire when `WAYLAND_DISPLAY` is
  set, Windows, macOS). A missing display is an error, not a barcode.
- `cbdesk-codec`: color VP8 via oxideav-vp8 0.2.7 (MIT, default features
  off, no system libvpx) and AV1 stills via rav1e (assembler and git tooling
  off) and rav1d-safe (AGPL-3.0-only). The first VP8 frame is a keyframe. A
  later frame can be a zero-motion inter frame. Negotiation prefers VP8.
- `cbdesk-fs`: resumable uncompressed files under a path jail. zstd is not
  linked.
- `cbdesk-audio`: a synthetic Opus tone and a 40–120 ms jitter buffer. No
  microphone is opened.
- `cbdesk-input`: enigo injection after the caller has granted Input, and
  `InputLog` for tests. Tests do not call `inject`.
- `cbdesk-cli`: `cbdesk host` and `cbdesk view`. After consent, frames stream
  until the viewer closes. `--window` shows them. `--ppm` writes the first
  frame. `--lan` / `--browse` find each other on the LAN. `--screen` captures
  the display. `--put`, `--tone`, `--audio`, and `--relay` are wired.
- `cbdesk-testkit`: re-exported barcode frames, recording injector,
  poisonable fake DHT.
- Netlab profile files for N0, N3, and N4. They do not create namespaces.
- Crate directories from the layout that have no code yet contain a README
  and are not workspace members.

## M0 items still open

The M0 exit criterion is not met. The plan's exit is a fork that builds on
three desktops. There is no fork here. CI being green on this workspace is the
substitute, not that exit.

- GitHub organization, branch protection, the DCO GitHub App, and Discussions.
  The owner checklist is `docs/dev/github-setup.md`.
- Trademark search (R-012). `docs/brand/` says the search was not done.
- UI wireframes S-01 through S-05.
- Import of the other code base, copyright headers from that import, and a
  build of that imported tree.
- `ubuntu-22.04`, Linux ARM, Windows ARM, macOS x86_64, Android, and iOS are
  not in CI.
- CodeQL, gitleaks, OSV-Scanner, `cargo-machete`, and `cargo-semver-checks`
  are not wired. Machete would also flag the intentional version pins in
  `cbdesk-crypto` and `cbdesk-proto`; those pins keep transitive crates on
  releases this tree has compiled. iroh adds a second `ed25519-dalek` and a
  second `getrandom`, which `cargo-deny` warns about and does not fail.
- No second maintainer, so the two-reviewer rule for crypto and networking is
  not enforceable yet.
- No external security review. The threat model is a draft written with the
  code.

## M1 items started

- Device identity in the OS keychain when that store answers within the
  caller's timeout. A missing entry is created. A store that does not answer
  is `KeyStoreUnavailable`, and the CLI keeps a new identity only in memory.
  The seed is not written to a file and keychain errors are not logged,
  because some of them attach the secret bytes.
- Session messages for hello, grant, ticket, DHT, input events, and clipboard
  text. A view frame is a unidirectional QUIC stream opened only after
  `authorize(View)`. An input event is a separate unidirectional stream
  accepted only after `authorize(Input)`. Clipboard text is a bidirectional
  stream accepted only after `authorize(Clipboard)`. A side stream before the
  grant, or a clipboard or input stream without its grant, is
  `PROTOCOL_VIOLATION`.
- `QuicEndpoint` on iroh 1.4.1 via `presets::Minimal`. The default relay
  list is empty, which selects `RelayMode::Disabled`. No n0 discovery
  service is installed. T-NET-001 is a loopback echo; the endpoint resolver
  must record no DNS names. It is not a packet capture of the whole machine.
  A URL the operator configured, and that is not a forbidden suffix, can be
  installed. `RelayMode::Default` is not selected. A relay named only on a
  ticket is ignored. HTTPS net-report probes run only when that list is
  non-empty, and only against those relays. A loopback relay test echoes
  one probe with UDP transports cleared. Symmetric NAT is unmeasured. The
  test relay is iroh's test helper, not a process this project ships.
- A verified invite ticket dials `direct_addrs` through that endpoint, then
  a relay this endpoint was already configured to use. The connection is
  handed to `HostSession`. The ticket is not a grant.
- LAN announce and browse. Public networks do not send. The datagrams are the
  tested mDNS TXT record and the `CBDS` broadcast body. `bind_multicast`
  joins `224.0.0.251` and binds port 5353 when that port can be shared.
  A busy 5353 still sends to the group and listens on an ephemeral port.
  The CLI passes a private profile only when the operator sets `--lan`. It
  does not read the OS network category.
- Consent policy in the library, including the 30 second approval timeout.
  The production wait is `wait_for_approval`. Loopback tests fire that same
  close path without sleeping 30 wall-clock seconds. The CLI still waits 3
  seconds on a first contact before `--approve` can grant.
- Synthetic capturer, a VP8 barcode stream, and an AV1 fallback. The live
  path sends color YUV at the capture's even size. The pacer targets 33 ms
  and adapts the VP8 quantizer. This tree has no 1080p timing measurement.
  Ubuntu CI proves the X11 path under Xvfb, including a viewer window that
  opens and paints one local frame. The Wayland ScreenCast path is compiled
  and selected when `WAYLAND_DISPLAY` is set; CI does not run it. Windows
  and macOS compile the capturer and the window. A missing display is
  `Unavailable` unless `CBDESK_REQUIRE_DISPLAY` is set. T-MED-001 selects
  VP8 when that is the only shared video codec. T-MED-004 here is the short
  barcode, including one inter frame at quantizer 0. It is not a 60-second
  soak.
- A scripted pointer and Enter are host stdout lines after an Input grant.
  The window maps pointer and key events the same way. The check is that
  QUIC delivery, not an injected operating-system click.
- File transfer resumes in `cbdesk-fs`. The CLI `--put` test stores
  `hello` after `file_write` and `--allow-high-risk`. Audio `--tone` reaches
  a viewer that passed `--audio`. Integration tests set
  `CBDESK_EPHEMERAL_IDENTITY=1` so two processes do not load one OS keychain
  identity and refuse to dial each other. Same-machine browse dials loopback
  before the multicast source address.

## Test ids this pass actually locks

| Id | What the test covers | What it does not cover |
| --- | --- | --- |
| T-NET-001 | Loopback QUIC echo on an endpoint with no n0 relays, no n0 discovery, and a resolver that recorded no names. Forbidden relay suffixes still fail before bind. A verified ticket dial does not query DNS for its relay URL. A separate test installs one operator URL and, on loopback only, echoes a probe with UDP off | A capture of every packet the runner sends, including traffic to GitHub. Symmetric NAT. A relay process shipped by this project |
| T-AUTH-018 | An unanswered approval closes at 30 seconds and the controller receives `TIMEOUT_USER_RESPONSE`. The 30 second sleep is the production wait; the loopback test uses that close path with a ready deadline | A visible approval window |
| T-AUTH-011 | Fifth failure locks 60 seconds, later failures double up to one hour | Password UI |
| T-AUTH-012 | Counters are per device id and per IPv4 /24 or IPv6 /56 | Distributed clients |
| T-AUTH-015 | A known trusted key can skip the prompt when the policy says so, and the grant is the safe set | A real trust store |
| T-AUTH-019 | One consent prompt per minute; three rejects silence the peer for 24 hours | A real window |
| T-PERM-002 | `authorize` refuses a permission that was not granted | A file browser UI |
| T-PERM-006 | Media and `authorize` fail before the grant. An echo probe is answered with `PROTOCOL_VIOLATION` and is not reflected. A unidirectional stream before the grant is the same close. After `authorize(View)`, frame streams are delivered. After `authorize(Input)`, an input event is returned and is not injected by the test. After `authorize(Clipboard)`, text is delivered. File and audio follow their grants and close the session without them. Clipboard, input, file, or audio without the grant closes the session | Moving the real pointer |
| T-MED-001 | Both peers offering only VP8 select VP8. VP8 outranks AV1 when both are offered. AV1 is selected when VP8 is absent | VP9, H.264, and H.265 |
| T-MED-004 | A short VP8 stream keeps the barcode readable at quantizer 0, including one zero-motion inter frame. Color sampling keeps a gray barcode gray | A 60-second soak, a hardware encoder, and a measured 1080p frame rate |
| T-UI-004 | First contact cannot be approved before 3 seconds | A widget |
| T-UI-005 | `indicator_can_be_disabled` is constant false | Settings UI, CLI |
| T-UI-007 / T-I18N-001 | Catalog keys match across Turkish and English, including every known error | A rendered UI |
| T-I18N-004 | Turkish short-code casing | A spoken usability test |

## Recommended next run

Measure the live pacer on a real display and record the frame interval.
The target is 33 ms at the capture's even size. Wayland ScreenCast is
compiled and untested in CI; a portal-capable lab would be the proof, not
an Xvfb job. A symmetric NAT measurement needs two machines and a relay the
operator runs outside this repository.

Commit `baef235` is red on macOS and Windows: the OS keychain made the two
CLI processes the same device, and LAN browse dialed the multicast source
address. This tree sets `CBDESK_EPHEMERAL_IDENTITY=1` in those tests and
dials loopback first. Those jobs have not run on this commit yet.

Still out of that run: hardware codecs, a microphone, the Flutter UI,
compressed file chunks, SPAKE2, OPAQUE, and DHT blinding. ADR-007 and
ADR-008 are still Proposed. The fork import stays closed for capture,
codec, and input (ADR-015). M0's fork-builds exit stays unmet.

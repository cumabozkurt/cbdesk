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
  `macos-14`. Flutter analyze and widget tests run on those three. Ubuntu
  also builds the Linux debug shell. This workflow also builds the Windows
  and macOS debug bundles and an arm64 Android debug APK. CI on `c01d0fb`
  is green. `80534bf` is not: the rebrand scan rejected the shell bridge
  class name, and the Windows `view_remote` tests timed out after the host
  main thread overflowed its stack. `2febc4d` fixed those two, and the Rust
  jobs passed. Its Flutter Windows and macOS bundle jobs failed. The plugin
  directory is a symlink, and climbing it looked for the crate under the
  ephemeral directory. `46337c5` resolves that link. Flutter on Windows
  then linked `cbdesk_ffi.dll`. Flutter on macOS built the static archive
  and the link failed for CoreAudio, AudioToolbox, CoreMedia,
  SystemConfiguration, and Carbon. `b01e162` names those frameworks and
  the app builds. `617fb1e` still ran Apple `nm` on the static archive.
  Xcode 15.4 cannot read the Rust 1.91 object files inside it. The
  following commit checks the linked framework in the app, which is a
  Mach-O. It is not green until Actions finishes.
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
  later frame of the same size is a motion-compensated inter frame
  (`encode_p_frame_multi_ref`, diamond search). A 48×16 test checks a block
  that moved 16 pixels at quantizer 96. Negotiation prefers VP8.
- `cbdesk-fs`: resumable uncompressed files under a path jail. zstd is not
  linked.
- `cbdesk-audio`: Opus through rusty-opus and a 40–120 ms jitter buffer.
  `--tone` does not open a device. `--mic` and `--speaker` open the default
  devices through cpal 0.15.3. A missing device is an error. This tree does
  not record that a person heard the stream.
- `cbdesk-input`: enigo injection after the caller has granted Input, and
  `InputLog` for tests. Tests do not call `inject`.
- `cbdesk-cli`: `cbdesk host` and `cbdesk view`. After consent, frames stream
  until the viewer closes. `--window` shows them. `--ppm` writes the first
  frame. `--lan` / `--browse` find each other on the LAN. `--screen` captures
  the display. `--put`, `--tone`, `--audio`, `--mic`, `--speaker`, and
  `--relay` are wired. `--tone` and `--mic` cannot be combined.
  `--screen` still captures the primary monitor. `list_monitors` and
  `capture_picture_on` can name another reported display. This workspace
  did not attach a second physical monitor, so that path was not observed
  on two screens. The live host does not take a monitor index.
- `cbdesk-desk`: device id, short code, signed ticket locator, consent,
  block list, address book, and a live session after consent. The two words
  confirm pairing through SPAKE2 before the consent prompt. They do not
  locate a computer and they are not stored in the ticket. A wrong code
  does not open the prompt. The host still approves, and the address book
  does not skip the first-contact delay. With no queued picture, the host
  encodes `capture_picture`. A queued preview or `submit_frame` is used
  first so the 48×16 motion test does not encode a full display. Input,
  clipboard text, one audio packet, and file bytes ride the granted
  streams. The manual clock, the preview queue, and the input-record flag
  are not exported.
- `cbdesk-ffi`: flutter_rust_bridge 2.12.0. `api.rs` forbids unsafe.
  Generated glue is the unsafe file.
- `app/desktop`: Flutter 3.47.7 shell for desktop and Android. Home,
  connect, consent, remote view, file panel, address book, and settings, in
  Turkish and English. The indicator is an edge and a disconnect panel
  above the pages and has no off switch. Connect takes a ticket and an
  optional short code. The remote view paints the decoded frame and sends
  a click, Enter, clipboard text, and a microphone switch. The file panel
  stages a name and sends bytes. Android shares the screen only after the
  system capture dialog; the longer side is at most 960. Widget tests use
  a fake desk and do not load the native library.
- `cbdesk-testkit`: re-exported barcode frames, recording injector,
  poisonable fake DHT. Random bytes are fed to the session decoders in
  `garbage_does_not_panic`. That is not cargo-fuzz or OSS-Fuzz, and
  neither was run. Ignored codec timings are described in
  `docs/dev/fuzz-and-bench.md`. Criterion is not a dependency.
- Netlab profile files for N0, N3, and N4. They do not create namespaces.
- Crate directories from the layout that have no code yet contain a README
  and are not workspace members.

## Network, password, and later-phase pass

This pass stays on `cursor/m0-foundation-0838`. The plan checkboxes in
`docs/MASTER_PLAN.md` are still unchecked.

- `cbdesk net-check` and `cbdesk doctor` print a local UDP and address-class
  report. CGNAT is `100.64.0.0/10`. Symmetric NAT, UPnP, DHT, and hole
  punching are explicitly not measured. `--loopback` connects two endpoints
  on this machine and says a direct path there is not a NAT hole punch.
  The test locks that. `classify_mappings` is the endpoint-dependent rule
  and is fed by a fixture, not by a live NAT. See `docs/dev/nat-measurement.md`.
- DHT blinded publish still returns `BlindingPendingReview`. A long-term
  record's `signer_pk` is the device id, and `dht_linkability` says so.
  Isolated mode refuses a relay and refuses DHT publish. ADR-007 stays
  Proposed. No blinding construction was added.
- Permanent passwords use `opaque-ke` 4.0.1 with Argon2id (19 MiB, 2 passes,
  1 lane). Host functions do not take the password. A test checks the
  password bytes are absent from the OPAQUE messages. zxcvbn score must be
  at least 3 and the password must be at least 10 characters. The CLI does
  not enroll a password. TOTP is HMAC-SHA1 and matches the RFC 6238 SHA-1
  vectors. It is not wired to the consent prompt. Neither composition has
  had an external review.
- Opus `software_loopback` encodes, buffers, and decodes a tone without
  opening a device. The mean absolute error in that test is below 0.2.
  Nobody has confirmed they heard it.
- Software VP8 at 1920×1080, two frames, quantizer 32, on this machine:
  19577 ms in debug and 416 ms in release (29550 bytes, one keyframe).
  Release is about 208 ms per frame, under 5 frames per second, not 30.
  The damage-path numbers and the ffmpeg feature flags are in the next
  section. 4:4:4 is refused. Quality presets are quantizer indexes.
- Wayland ScreenCast remains unrun in CI. The manual plan is
  `docs/dev/wayland-screencast.md`.
- Session gates still do not touch the operating system by themselves.
  They return an `OsPermit`. `cbdesk-hostops` consumes that permit. The
  live `cbdesk host` and desk session do not call it. Permission presets
  that omit high-risk permissions, the in-memory BLAKE3 access log, session
  and idle deadlines, side-switch that needs both consents, whiteboard
  strokes that need the indicator, and a Wake-on-LAN payload that is not
  sent stay library checks.
- Still blocked, and not pretended otherwise: GitHub organization settings,
  trademark search, an external audit, signed installers and store listings,
  an Apple or Google developer account, a TUF root (ADR-013), an
  organization certificate parser (ADR-011), a CRDT library (ADR-012), DHT
  blinding (ADR-007), a public relay, a live symmetric NAT, a
  72-hour soak, and a second machine that hashes the same binary.
  `docs/dev/deployment.md` says which templates exist and which commands
  were not run.

## Performance, operating-system effects, and release templates

Actions on `5602049` finished red. The policy job failed on
RUSTSEC-2026-0387 (`crypto-bigint` 0.5.5 via `opaque-ke` 4.0.1 and
`elliptic-curve` 0.13.8). Clippy, tests, and Flutter were green on Ubuntu
24.04, Windows 2022, and macOS 14. The last fully green run remains
[39919713548](https://github.com/cumabozkurt/cbdesk/actions/runs/39919713548)
on `e69c781`. `deny.toml` and `.cargo/audit.toml` ignore that one advisory.
The published fix is `crypto-bigint` >= 0.7.6, which `elliptic-curve` 0.13
does not use, and no stable `opaque-ke` has moved. `opaque-ke` 4.1.0
pre-releases are not used. Delete both ignores in the same commit that
bumps `opaque-ke`, and only after
`cargo tree -i crypto-bigint -p cbdesk-crypto` no longer lists 0.5.5.
This tree does not patch the arithmetic. `clipboard-win` 5.4.1 stays a
yanked warning.

- Damage tracking compares 64×64 tiles on two threads when the picture
  has at least two tile rows. Each 16-byte group is a `wide` 1.7.1 `u8x16`
  compare: SSE2 on x86_64, NEON on aarch64, scalar tail after that. This
  crate still has no `unsafe` block. It is not an AVX kernel. An unchanged
  picture is not encoded and is not sent. A dirty
  region at most a quarter of the picture, smaller than the frame on both
  axes, and at least 16 pixels after alignment, is cropped to 16 pixels and
  encoded as its own VP8 keyframe. The full-frame encoder is left alone.
  Anything larger stays on that inter encoder. The header sets
  `damage_width` and clears `keyframe` for a region. The desk and the CLI
  viewer place it with a separate decoder. A peer that ignores those fields
  must not be sent the crop. Release numbers from
  `vp8_1080p_damage_path` on this machine on 2026-10-25: full keyframe
  201 ms, unchanged repeat reported as 0 ms, 64×64 region 2 ms and 34
  bytes, tile scan 460 µs. A later release run of that same test, after
  the `u8x16` compare, printed full 200 ms, region 1 ms, scan 363 µs.
  `measure_1080p_paths` on the same day, same solid 1920×1080 picture:
  one keyframe 198 ms, four strips in series 196 ms, four strips in
  parallel 52 ms, half size 48 ms, quarter size 12 ms. The parallel and
  scaled rows are this solid picture only. 52 ms is still over the 33 ms
  slot. The strips are separate keyframes. `cbdesk host --strips N` (default
  1, at most 16) sends them when `N` is greater than 1 and the negotiated
  codec is VP8. The header then names the full canvas (`canvas_width`,
  `canvas_height`, `strip_index`, `strip_count`) and the viewer blanks that
  canvas before painting each band. The default stays on the damage encoder
  and omits those fields. A viewer that does not know them cannot place the
  bands. The desk host does not take `--strips`. `--profile
  balanced` and `--profile preview` scale a `--screen` capture before the
  existing encoder. `--synthetic` stays full size. None of this is 30
  frames per second, and a detailed desktop was not measured. A full-frame
  change on the default path is still about five frames per second.
  Details are in `docs/dev/media-measurement.md`.
- Hardware cargo features `hw-vaapi`, `hw-mediafoundation`,
  `hw-videotoolbox`, and `hw-mediacodec` are off by default. They shell out
  to ffmpeg for `vp8_vaapi`, `vp8_mf`, `vp8_videotoolbox`, and
  `vp8_mediacodec`. VA-API also needs a `/dev/dri/renderD*` node. There is
  no in-process hardware context. The quantizer is not forwarded. NVENC,
  AMF, and Quick Sync stay unlinked. No successful hardware encode was
  recorded. Ubuntu CI runs the VA-API feature test, which still has to
  fail closed when the device or the encoder is missing.
- `cbdesk-hostops` is the operating-system side of a permit. On Linux it
  grabs the X11 keyboard and pointer and maps a fullscreen black
  override-redirect window on the display the caller names. Drop releases
  the grab and destroys the window. Tests start a private Xvfb and do not
  use the ambient `DISPLAY`. Wayland is not handled. Windows calls
  `BlockInput` and releases it on drop. macOS creates a session event tap
  on a `CFRunLoop` and stops it on drop. A refused `BlockInput` or a null
  tap is `Unavailable`. This workspace did not record a successful grab on
  either system. The privacy cover is still the X11 window. ConPTY and a
  signed virtual display are not implemented.
- Recording writes `CBDesk recording` plus the notice text to a file the
  caller names, then tries `notify-send`. A desktop popup is recorded only
  when that command exits 0. The file is not an encrypted recording. The
  indicator still cannot be disabled, and a privacy permit cannot post the
  notice.
- The terminal opens a Unix pseudoterminal and starts `sh`, or on Windows
  starts `cmd.exe /Q` with pipes. ConPTY is not linked. Drop kills the
  child. The tunnel binds `127.0.0.1:0`, copies one read, and refuses a
  destination that is not loopback before it accepts. A test copies `ping`
  to another loopback listener and rejects `8.8.8.8:9`.
- `open_virtual_display` on Linux starts `Xvfb :N -screen 0 1280x720x24
  -nolisten tcp -ac` and kills it on drop. A missing binary is an error.
  Windows indirect display and a macOS virtual display stay unavailable.
  Signing for those drivers has not started. The live capture path does
  not call this function.
- `cbdesk-controller` depends only on `cbdesk-proto`. It reports the
  controller role and refuses host features. macOS CI runs
  `tools/ios/check-controller.sh`, which `cargo check`s that crate for
  `aarch64-apple-ios` when the iphoneos SDK is installed. This machine is
  not Darwin, so the script exited 0 without checking. There is no signed
  TestFlight build and no Apple Developer account in this tree.
- `tools/android/device-check.sh` talks to `adb` only when a device is in
  the `device` state. A missing `adb` or device exits 0.
  `CBDESK_ANDROID_DEVICE=1` makes that a failure. This machine has no
  `adb`. No phone was installed, and the system capture dialog was not
  accepted. The activity name in the script is
  `org.cbdesk.cbdesk/.MainActivity`.
- `tools/release/sbom.py --locked --check` listed 813 packages from
  `cargo metadata` (SPDX 2.3, `filesAnalyzed` false, a missing license is
  `NOASSERTION`). `tools/release/sign.sh` wrote a sha256 and, with
  `CBDESK_SIGN_KEY` unset, a note that begins `unsigned`. It exits 0. No
  key is in git. `tools/release/reproducible.sh` printed
  `SOURCE_DATE_EPOCH`, `rustc -vV`, and the sha256 of `Cargo.lock`. It does
  not compare two binaries, so this tree does not claim a reproducible
  build. `packaging/deb/build.sh`, the AppImage script, `make-dmg.sh`, the
  WiX source, and the Flatpak manifest are templates. The deb was not
  built here. AppImage exits if `linuxdeploy` is missing and does not
  download it. The DMG script exits on Linux. WiX was not compiled.
  Flatpak was not submitted to Flathub. `.github/workflows/release.yml` is
  `workflow_dispatch` only and uploads those unsigned Ubuntu artifacts.
  It does not run on this pull request.
- The session indicator is a Flutter semantics live region. The node's
  label includes the existing catalog title (`Session` / `Oturum`) and the
  sentence that the indicator cannot be turned off, plus the status text.
  Navigation icons use the existing labels. Text fields also use those
  existing strings as labels, the session border is excluded from the
  semantics tree, and a remote picture is an image button named with the
  existing remote-view sentence. No new ARB keys were added.
  The widget test checks the live-region flag and those Turkish substrings.
  Actions on `6a11de6` failed that test because it required the label to
  be exactly `Oturum`. Flutter is not installed in this environment.
  A person has not used a screen reader against the shell.

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
- `ubuntu-22.04`, Linux ARM, Windows ARM, macOS x86_64, and Android are
  not in CI. macOS CI type-checks `cbdesk-controller` for
  `aarch64-apple-ios`. That check is not an iOS app or a device.
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
  stream accepted only after `authorize(Clipboard)`. HTML in that text is
  reduced to plain text before it is sent or written. A line that is
  `cbdesk-secret` is not shared. That mark is not password detection.
  File paste is not implemented. A side stream before the grant, or a
  clipboard or input stream without its grant, is `PROTOCOL_VIOLATION`.
- `QuicEndpoint` on iroh 1.4.1 via `presets::Minimal`. The default relay
  list is empty, which selects `RelayMode::Disabled`. No n0 discovery
  service is installed. T-NET-001 is a loopback echo; the endpoint resolver
  must record no DNS names. It is not a packet capture of the whole machine.
  A URL the operator configured, and that is not a forbidden suffix, can be
  installed. `RelayMode::Default` is not selected. A relay named only on a
  ticket is ignored. HTTPS net-report probes run only when that list is
  non-empty, and only against those relays. A loopback relay test echoes
  one probe with UDP transports cleared. `cbdesk-relayd` is a separate
  loopback process with a self-signed certificate. Its test echoes one
  probe the same way. A live symmetric NAT is unmeasured. `cbdesk net-check`
  and the mapping classifier are described in `docs/dev/nat-measurement.md`.
  Details of the daemon are in `docs/dev/relayd.md`. No image was published.
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
  and adapts the VP8 quantizer. A 1080p debug timing is in
  `docs/dev/media-measurement.md` and is not 30 frames per second.
  The Flutter desk encodes that live capture when no picture is queued.
  The 48×16 motion pair is the queued test picture.
  Ubuntu CI proves the X11 path under Xvfb, including a viewer window that
  opens and paints one local frame. The Wayland ScreenCast path is compiled
  and selected when `WAYLAND_DISPLAY` is set; CI does not run it. Windows
  and macOS compile the capturer and the window. A missing display is
  `Unavailable` unless `CBDESK_REQUIRE_DISPLAY` is set. T-MED-001 selects
  VP8 when that is the only shared video codec. T-MED-004 here is the short
  barcode, including one inter frame at quantizer 0, and a 48×16 block that
  moved at quantizer 96. It is not a 60-second soak.
- A scripted pointer and Enter are host stdout lines after an Input grant.
  The window maps pointer and key events the same way. The check is that
  QUIC delivery, not an injected operating-system click.
- File transfer resumes in `cbdesk-fs`. The CLI `--put` test stores
  `hello` after `file_write` and `--allow-high-risk`. The Flutter file panel
  jails the name and sends the bytes on the QUIC file stream. Audio
  `--tone` reaches a viewer that passed `--audio`. `--mic` and `--speaker`
  open real devices. The shell switch calls the same microphone open on the
  controller. A missing device does not end the desk session. This tree
  does not record that a person heard the stream. Integration tests set
  `CBDESK_EPHEMERAL_IDENTITY=1` so two processes do not load one OS keychain
  identity and refuse to dial each other. Same-machine browse dials loopback
  before the multicast source address.

## Test ids this pass actually locks

| Id | What the test covers | What it does not cover |
| --- | --- | --- |
| T-NET-001 | Loopback QUIC echo on an endpoint with no n0 relays, no n0 discovery, and a resolver that recorded no names. Forbidden relay suffixes still fail before bind. A verified ticket dial does not query DNS for its relay URL. A separate test installs one operator URL and, on loopback only, echoes a probe with UDP off. `net-check` classifies local addresses and a loopback path test requires a direct path | A capture of every packet the runner sends, including traffic to GitHub. A live symmetric NAT. A relay process shipped by this project |
| T-AUTH-018 | An unanswered approval closes at 30 seconds and the controller receives `TIMEOUT_USER_RESPONSE`. The 30 second sleep is the production wait; the loopback test uses that close path with a ready deadline | A visible approval window |
| T-AUTH-011 | Fifth failure locks 60 seconds, later failures double up to one hour | Password UI |
| T-AUTH-012 | Counters are per device id and per IPv4 /24 or IPv6 /56 | Distributed clients |
| T-AUTH-015 | A known trusted key can skip the prompt when the policy says so, and the grant is the safe set | A real trust store |
| T-AUTH-019 | One consent prompt per minute; three rejects silence the peer for 24 hours | A real window |
| T-PERM-002 | `authorize` refuses a permission that was not granted | A file browser UI |
| T-PERM-006 | Media and `authorize` fail before the grant. An echo probe is answered with `PROTOCOL_VIOLATION` and is not reflected. A unidirectional stream before the grant is the same close. After `authorize(View)`, frame streams are delivered. After `authorize(Input)`, an input event is returned and is not injected by the test. After `authorize(Clipboard)`, text is delivered. File and audio follow their grants and close the session without them. Clipboard, input, file, or audio without the grant closes the session | Moving the real pointer |
| T-MED-001 | Both peers offering only VP8 select VP8. VP8 outranks AV1 when both are offered. AV1 is selected when VP8 is absent | VP9, H.264, and H.265 |
| T-MED-004 | A short VP8 stream keeps the barcode readable at quantizer 0, including one inter frame. A 48×16 test shifts a white block by 16 pixels at quantizer 96. Color sampling keeps a solid color dominant. Ignored tests time two 1080p frames and one damage path. Neither requires 30 fps | A 60-second soak and a successful hardware encode |
| T-UI-004 | First contact cannot be approved before 3 seconds. The shell keeps Accept disabled until the desk says the delay has elapsed | A wall-clock widget that sleeps 3 seconds |
| T-UI-005 | `indicator_can_be_disabled` is constant false. The shell has no hide control, paints the indicator above consent, and the settings page says it cannot be turned off | A tray icon on a logged-in desktop |
| T-UI-007 / T-I18N-001 | JSON catalog keys and Flutter ARB keys match across Turkish and English | A spoken usability test |
| T-I18N-004 | Turkish short-code casing | A spoken usability test |

Widget tests in `app/desktop/test` drive the shell with a fake desk. They
cover the home id and the pairing sentence, a short code without a ticket,
the consent delay flag, the high-risk opt-in, Reject focus, the missing
hide-indicator control, disconnect, the file-name jail, a file-byte send,
a click, Enter, clipboard text, the microphone switch, the address book,
and the English settings strings. They do not load `libcbdesk_ffi`. The
Rust bridge test refuses a short code that has no ticket and keeps
`indicator_can_be_disabled` false.
`flutter build linux --debug` on this machine produced a bundle that
contains `libcbdesk_ffi.so`. Starting that binary under Xvfb loaded the
library and wrote an identity file. That start is not a click-through of
the screens and it is not a session to another computer.
`flutter build apk --debug --target-platform android-arm64` on this
machine produced an APK whose `lib/arm64-v8a/libcbdesk_ffi.so` is the Rust
bridge. That APK was not installed on a device, and the system capture
dialog was not accepted. Windows and macOS bundles are in the workflow.
`46337c5` linked the Windows DLL. `b01e162` built `cbdesk.app` after the
pod linked AudioToolbox, CoreAudio, CoreMedia, SystemConfiguration, and
Carbon. `617fb1e` built the app again, and Apple `nm` exited on the static
archive because Xcode 15.4 cannot read those Rust 1.91 objects. The
following commit checks the linked framework inside the app. That check
is not proven until Actions finishes.

## Plan §18 ledger

The checkboxes in `docs/MASTER_PLAN.md` stay unchecked. This table is the
closing map for that section. It does not start another feature. Actions
on `6a11de6` ([40207159596](https://github.com/cumabozkurt/cbdesk/actions/runs/40207159596))
were red: the Flutter test required the live-region label to be exactly
`Oturum`, and non-Linux builds called `mapped_size` on `Unavailable`.
macOS also rejected an unused `mut` on the PTY flags. The policy job and
Ubuntu clippy and test on that run were green. `0abb194` then left only
Windows clippy and test red: the Unix ffmpeg stub left `Write` and
`tempfile_dir` unused. Those two are now compiled on Unix only. A green
result belongs to the Actions run on this commit. This paragraph does not
invent one.

| Plan item | State | What is in the tree |
| --- | --- | --- |
| M0 GitHub org, branch protection, CODEOWNERS, DCO app, Discussions | Owner | The repo is `cumabozkurt/cbdesk`. CI runs a local DCO script. Org settings, protection, the GitHub DCO App, and Discussions are not configured from here. |
| M0 trademark search and domain (R-012) | Owner | `docs/brand/` says the search was not done. |
| M0 RustDesk fork and rebrand | Absent | ADR-002 started from empty crates. ADR-015 closes a later import of capture, codec, and input. `tools/rebrand-check.sh` runs in CI. There is no fork to clean. |
| M0 licenses, NOTICE, SPDX headers, THIRD_PARTY_LICENSES | Done | Those files are in the tree. The release SBOM is a cargo-metadata list, not a signed third-party audit. |
| M0 ADR-001 through ADR-006 accepted | Done | Those six are Accepted. ADR-007, ADR-011, ADR-012, and ADR-013 stay Proposed. |
| M0 CI: lint, test, and build on Windows, macOS, and Linux | Partial | The workflow covers fmt, deny, audit, clippy, tests, and Flutter bundles on `ubuntu-24.04`, `windows-2022`, and `macos-14`. `6a11de6` failed as named above. |
| M0 testkit and netlab profiles N0, N3, N4 | Partial | The crate and the profile files exist. The profiles do not create network namespaces. |
| M0 UI wireframes S-01 through S-05 | Absent | No Penpot or Figma frames are in the tree. |
| M0 threat model v0.1 | Partial | `docs/threat-model.md` is a draft written with the code. It is not an external review. |
| M0 exit: the fork builds on three desktops, the rebrand scan is clean, ADRs are merged | Absent | There is no fork. A clean rebrand scan is the absence of that tree. ADR-001 through ADR-006 are accepted. A green CI run is not this exit. |
| M1 iroh without n0 defaults, direct IP, LAN mDNS, invite ticket | Partial | Loopback QUIC, LAN announce and browse, and a verified ticket dial are tested. A live symmetric NAT is not measured. A loopback path is not a hole punch. |
| M1 T-NET-001 in CI | Done | Loopback echo on an endpoint with no n0 relays and a resolver that recorded no names. It is not a capture of every packet the runner sends. |
| M1 identity key and OS keychain on three desktops | Partial | The keyring path is compiled. A store that does not answer keeps a new identity in memory. CI does not prove three logged-in keychains. |
| M1 RustDesk `message.proto` on the new transport, with VP8/VP9 and keyboard and mouse | Absent | The protocol is clean-room, not `message.proto`. VP8 and AV1 exist. VP9 does not. A scripted pointer and Enter are host stdout after an Input grant. |
| M1 simple interactive approval | Partial | The Flutter consent panel and `cbdesk host --approve` exist. This is not a finished approval window. |
| M1 synthetic capturer, recording injector, Linux to Windows end to end | Partial | Synthetic frames and the recording injector exist. No Linux-to-Windows session was run. |
| M1 exit: LAN, N0 to N3, T-NET-002/003, T-DISC-001, T-MED-004, demo video | Partial | Same-machine LAN tests exist. Netlab namespaces were not created. T-MED-004 is a short barcode, one 48×16 inter frame, and ignored 1080p timings. It is not a 60-second soak. There is no demo video. |
| M2 short code and PAKE, with a cryptography review | Partial | The desk host uses `spake2` 0.4.0 before consent. The CLI host does not. ADR-008 is Accepted. There has been no external review. |
| M2 mainline DHT, blinding behind a flag | Partial | Records can be published in the clear. Blinded publish returns `BlindingPendingReview`. ADR-007 stays Proposed. |
| M2 embedded community relay and relay discovery | Partial | An operator-configured relay URL can be installed. `cbdesk-relayd` binds loopback only, prints a self-signed `https://` URL, and a test echoes one probe with UDP off. It is not a public relay. A symmetric NAT was not measured. n0 defaults stay off. |
| M2 authentication, permission engine, connection manager | Partial | Consent, grants, and `authorize` exist. There is no separate connection-manager product. |
| M2 Flutter screens S-01 through S-06, S-10, S-11 | Partial | Home, connect, consent, remote view, files, address book, and settings exist. They are not those wireframe ids. |
| M2 fraud warning and an indicator that cannot be disabled | Done | The shell and the library both keep the indicator on. The indicator is a semantics live region. |
| M2 Turkish and English | Partial | Catalogs cover the shell strings that exist, in both languages. The language switch shows `Türkçe` and `English` from the ARB files, including when the UI is the other language. There is no spoken usability test and no video tutorial. |
| M2 portable Windows EXE, unsigned DMG, AppImage | Partial | `make-dmg.sh` and the AppImage script are templates and were not run. No Windows EXE installer was built. |
| M2 exit: desktop smoke matrix, the named test ids, an alpha release | Absent | Version 0.0.0 is not a release. The smoke matrix was not run by a person. |
| M3 service, agent, and netd split on three desktops | Absent | One host process. The split in §5.3 is not built. |
| M3 OPAQUE password and TOTP | Partial | `opaque-ke` 4.0.1 and HMAC-SHA1 TOTP are libraries. The CLI does not enroll a password. TOTP is not on the consent prompt. RUSTSEC-2026-0387 is ignored until a stable `opaque-ke` no longer depends on `crypto-bigint` 0.5.5. |
| M3 hardware codecs and a blocklist | Partial | `hw-vaapi`, `hw-mediafoundation`, `hw-videotoolbox`, and `hw-mediacodec` shell out to ffmpeg and are off by default. No successful hardware encode was recorded. There is no codec blocklist. |
| M3 audio | Partial | Opus and a cpal device open exist. This tree does not record that a person heard the stream. |
| M3 `cbdesk doctor`, CLI v1, `cbdesk-relayd`, Docker image | Partial | `cbdesk doctor` prints a local address report. It is not CLI v1. `cbdesk-relayd` is the loopback binary in `docs/dev/relayd.md`. `packaging/relayd/Dockerfile` was not built and no image was published. |
| M3 MSI, pkg, deb, rpm, Flatpak, winget, Homebrew, AUR | Partial | Unsigned deb, WiX, Flatpak, DMG, and AppImage templates exist. The deb was not built here. rpm, winget, Homebrew, AUR, and a signed pkg are absent. |
| M3 DHT blinding on by default after review | Absent | Blinded publish still fails closed. There has been no review. |
| M3 usability round T-UX-001 through T-UX-003 | Absent | No person ran that round. |
| M3 exit: service, NAT, auth, and media tests, performance targets, a beta channel | Absent | A 1920×1080 software keyframe measured 201 ms, and a later solid-frame run measured 198 ms. Parallel strips of that solid frame took 52 ms and a quarter-size preview took 12 ms. Those are not the live inter-frame path and not a desktop. The 30 frame-per-second and glass-to-glass targets are not met. No beta channel exists. |
| M4 Android host: MediaProjection, accessibility service, foreground service, F-Droid | Partial | CI builds an arm64 debug APK. Capture waits for the system dialog. That dialog was not accepted on a device. No accessibility service and no F-Droid submission. |
| M4 iOS controller on TestFlight | Partial | macOS CI type-checks `cbdesk-controller` for `aarch64-apple-ios`. There is no signature, no TestFlight build, and no Apple Developer account. |
| M4 CRDT sync of "my devices" | Absent | ADR-012 is not implemented. |
| M4 terminal, tunnel, file paste, session recording, privacy mode | Partial | X11 grab and cover, a PTY or `cmd.exe` pipes, a loopback tunnel, a notice file, and Xvfb exist and are tested. Windows `BlockInput` and a macOS session event tap are compiled and release on drop. This workspace did not record a successful grab on either system. The cover is still X11. The live `cbdesk host` session does not call them. File transfer is a QUIC stream under a path jail. Recording is not an encrypted file. |
| M4 quick-support binary | Absent | Quick support stays banned in the session policy. |
| M4 exit: mobile matrix and the terminal, tunnel, and admin tests | Absent | No device matrix was run. |
| M5 organization root, certificates, policy documents, CRL | Absent | ADR-011 stays Proposed. No parser is linked. |
| M5 GPO, Intune, Jamf, and Ansible examples | Absent | `docs/dev/deployment.md` records that those tools were not pointed at a package. |
| M5 Windows indirect display, with signing started (R-007) | Partial | Linux `open_virtual_display` starts Xvfb. Windows indirect-display signing has not started. macOS has no virtual-display entitlement here. |
| M5 whiteboard, side switch, 4:4:4 | Partial | Whiteboard strokes and side switch are library gates. `require_chroma` of 4:4:4 fails. |
| M5 exit: auth and media tests, a volunteer pilot | Absent | No pilot was run. |
| M6 independent security audit | Owner | Not commissioned. |
| M6 performance work until every NFR holds | Partial | The damage scan uses `wide` `u8x16`. `encode_strips` can encode horizontal bands on several threads, and `cbdesk host --strips` can send them as separate keyframes. `--profile balanced` and `preview` box-filter a screen capture. The numbers in `docs/dev/media-measurement.md` are one machine and one synthetic solid picture. The NFR targets are not met. |
| M6 72-hour soak and chaos tests | Absent | Not run. |
| M6 finished Turkish and English docs, plus video tutorials | Partial | User notes and the shell catalogs exist in both languages. The book has a limits page for owner-owned and absent work. There are no video tutorials. |
| M6 reproducible build checked by someone else | Partial | `tools/release/reproducible.sh` records `SOURCE_DATE_EPOCH`, `rustc -vV`, and the `Cargo.lock` digest. No second machine hashed a binary. |
| M6 notice to antivirus vendors (R-011) | Owner | Not sent. |
| M6 exit: release candidate and version 1.0.0 | Absent | The version is 0.0.0. Nothing was published. |
| M7 Tor, multipath, hardware keys, HDR, pen, gamepad, KVM, protocol gateways, ADB, notification bridge, iOS view-only host, web client, address-book import | Absent | None of these are started. |

RUSTSEC-2026-0387 stays ignored while `opaque-ke` 4.0.1 pulls `crypto-bigint`
0.5.5 through `elliptic-curve` 0.13.8. The published fix is
`crypto-bigint` >= 0.7.6. Remove the id from `deny.toml` and from
`.cargo/audit.toml` in the commit that moves to a stable `opaque-ke` whose
tree no longer contains 0.5.5. Confirm with
`cargo tree -i crypto-bigint -p cbdesk-crypto` before deleting the ignore.
A 4.1.0 pre-release is not that commit. The arithmetic is not patched in
this tree.

## Stop

The user guide source is `docs/book`. `.github/workflows/pages.yml` builds
it on every push to `main` and deploys with GitHub Pages when the repository
source is GitHub Actions. The site does not offer a download. This commit
adds `cbdesk-relayd`, a loopback-only self-signed relay. It does not
measure a symmetric NAT, and it does not publish an image. It does not add
a Windows or macOS input grab or a device test.
Wayland ScreenCast remains the manual plan in
`docs/dev/wayland-screencast.md`. The live host still
does not hold the X11 grab, the privacy window, the shell, or the tunnel.
`clipboard-win` 5.4.1, pulled by arboard 3.7.0, stays a yanked warning.

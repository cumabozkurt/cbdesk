# Changelog

This file follows Keep a Changelog. The project uses semantic versioning.
Version 0.0.0 is not a release.

## [Unreleased]

### Fixed

- The macOS event tap links CoreGraphics and CoreFoundation in separate
  `extern` blocks. One stacked pair of `#[link]` attributes fails
  `clippy::duplicated_attributes`.
- The session-indicator widget test accepts the full live-region label.
  Actions on `6a11de6` required that label to be exactly `Oturum`.
  Non-Linux builds no longer call `mapped_size` on the unavailable cover,
  and the Unix PTY flags are not `mut` on macOS. The ffmpeg stub helpers
  are compiled only on Unix, which is what Windows clippy on `0abb194`
  rejected.

### Added

- Session decoders are exercised with random bytes in
  `garbage_does_not_panic`. That is not cargo-fuzz or OSS-Fuzz. An ignored
  test times four VP8 strips of a 64×64 picture. See
  `docs/dev/fuzz-and-bench.md`. Criterion is not a dependency.
- `cbdesk-relayd` binds a loopback iroh relay, prints a self-signed
  `https://` URL, and refuses any other address. A test echoes one probe
  with UDP off. That is not a symmetric-NAT measurement. The Docker sketch
  was not built and no image was published. n0 relay defaults stay off.
- Software VP8 can compare damage tiles with `wide` 1.7.1 `u8x16` (SSE2 or
  NEON, no `unsafe` in this crate) and can encode horizontal strips on
  several threads. `--profile balanced` and `--profile preview` box-filter
  a `--screen` capture by 2 or 4. The default `full` profile does not
  scale, and `--synthetic` rejects the other two. A release run of one
  solid 1920×1080 picture on this machine took 198 ms as one keyframe,
  52 ms as four parallel strip keyframes, 48 ms at half size, and 12 ms
  at quarter size. The strips are not sent on the live session. This is
  not 30 frames per second. Numbers are in `docs/dev/media-measurement.md`.
- An mdBook user guide in `docs/book`, in Turkish and English, with a
  bilingual landing page. `.github/workflows/pages.yml` builds it on every
  push to `main` and deploys with GitHub Pages. There is still no download.
- The Flatpak template id is `org.cbdesk.CBDesk`. It has not been submitted
  to Flathub.
- Damage tracking on the software VP8 path. Unchanged pictures are not
  encoded. A small dirty region is a separate keyframe with `damage_width`
  set and `keyframe` cleared. The release measurement on this machine is in
  `docs/dev/media-measurement.md`: 201 ms for a 1920×1080 keyframe, under
  1 ms reported for the repeat, 2 ms for a 64×64 region. That is not 30
  frames per second. `hw-vaapi`, `hw-mediafoundation`, `hw-videotoolbox`,
  and `hw-mediacodec` shell out to ffmpeg and are off by default. No
  in-process hardware context is linked, and no successful hardware encode
  was recorded.
- `cbdesk-hostops` performs a consented X11 grab and black window, a
  recording notice file, a Unix pseudoterminal or Windows `cmd.exe` pipes,
  and a loopback TCP forward. `open_virtual_display` starts `Xvfb` on
  Linux. The live `cbdesk host` session does not call these. Windows
  calls `BlockInput` and macOS creates a session event tap; both release
  on drop. A refused call is `Unavailable`, and this workspace did not
  record a successful grab on either system. The privacy cover is still
  X11. ConPTY and a signed virtual display are not implemented.
- `cbdesk-controller` is the view-side crate macOS CI type-checks for
  `aarch64-apple-ios`. It is not a signed iOS build.
  `tools/android/device-check.sh` talks to `adb` only when a device is
  present. No phone was tested.
- Unsigned release templates: SPDX from `cargo metadata`, a checksum script
  that stays unsigned without `CBDESK_SIGN_KEY`, a build-input record that
  does not claim bit-reproducibility, deb / AppImage / DMG / WiX / Flatpak
  sketches, and a `workflow_dispatch` release workflow. No key is in git
  and no installer was signed. RUSTSEC-2026-0387 is ignored in `deny.toml`
  and `.cargo/audit.toml` because `opaque-ke` 4.0.1 cannot move off
  `crypto-bigint` 0.5.5. The arithmetic is not patched.
- The Flutter session indicator is a semantics live region labeled with the
  existing Session / Oturum string. Navigation icons use the existing
  labels. No new catalog keys were added. A screen reader was not used.
- `cbdesk net-check` and `cbdesk doctor` report local UDP binds, CGNAT
  (`100.64.0.0/10`), and an address class. Symmetric NAT, UPnP, DHT, and
  hole punching are reported as not measured. `--loopback` records an iroh
  path on this machine and says that path is not a NAT hole punch. A
  mapping classifier covers the endpoint-dependent case in a unit test.
  See `docs/dev/nat-measurement.md`.
- DHT publish under a blinded key still fails closed. Long-term records
  are documented as linkable to the device id. Isolated mode rejects a
  relay and a DHT publish. ADR-007 stays Proposed.
- Permanent passwords use `opaque-ke` 4.0.1 with Argon2id (19 MiB, 2
  passes, 1 lane). The password is not an argument of the host functions.
  Length is at least 10 characters and the zxcvbn score is at least 3.
  The CLI does not enroll a password. TOTP is HMAC-SHA1 (RFC 6238) and is
  not attached to consent. Neither the OPAQUE composition nor the SPAKE2
  transcript has had an external review.
- A software Opus loopback encodes, buffers, and decodes a tone without
  opening a device. A 1920×1080 software VP8 timing is recorded in
  `docs/dev/media-measurement.md` and is not 30 frames per second.
  4:4:4 is refused. Hardware features are described in a later entry.
- Library gates for permission presets, an in-memory BLAKE3 access log,
  session deadlines, side switch, whiteboard strokes, and a Wake-on-LAN
  payload that is not sent. Privacy, block-input, recording, terminal,
  tunnel, and the virtual display now have operating-system functions in
  `cbdesk-hostops` and `cbdesk-capture`. The live session does not call
  them. Signed installers, an external audit, and TUF are not done. See
  `docs/dev/deployment.md` and `docs/dev/wayland-screencast.md`.
- Short-code pairing with the published `spake2` 0.4.0 crate. The password
  is the two lowercase words. The slot and both device keys are transcript
  only. The words are not written into the ticket. A desk host requires the
  exchange before consent. A wrong code does not open the prompt. The host
  still approves. The CLI host does not run this exchange. The transcript
  has not had an external review.
- The Flutter shell sends the live session. A desktop host encodes
  `capture_picture` when no picture is queued. The remote view forwards a
  click and Enter after an Input grant, clipboard text after a Clipboard
  grant, and a microphone switch after an Audio grant. The file panel sends
  bytes over QUIC after the jail accepts the name. A missing microphone
  does not end the session. This tree does not record that a person heard
  the audio. Android capture uses the system dialog and does not create a
  virtual display until that dialog is accepted. The longer side is scaled
  to at most 960.
- CI builds the Windows and macOS debug shells and an arm64 Android debug
  APK, and checks that each links `libcbdesk_ffi`. The Flutter plugin
  directory is a symlink. The crate path is resolved to the physical
  directory before the build looks up `crates/cbdesk-ffi`. The macOS pod
  links AudioToolbox, CoreAudio, CoreMedia, SystemConfiguration, and Carbon
  with that archive. The symbol check reads the linked `cbdesk_ffi`
  framework inside the app. Apple `nm` from Xcode 15.4 cannot read the
  Rust 1.91 object files inside the static archive. The macOS bundle is
  not proven until Actions finishes.
- VP8 inter frames and color. The first frame is a keyframe. A later frame
  of the same size is a motion-compensated inter frame: oxideav-vp8 diamond
  search (whole, half, and quarter pixel). A 48×16 test shifts a white block
  by 16 pixels at quantizer 96 and checks the moved columns. The barcode
  test at quantizer 0 still reads that barcode on an inter frame. The live
  path keeps the capture's even size. The pacer targets 33 ms and raises the
  quantizer when encode time, loss, or delay says so. A later note in
  this changelog records a 1080p timing that is not 30 frames per second.
- A scripted pointer and Enter travel over QUIC and the host prints them
  after an Input grant. The window uses that mapping. The host sends an
  empty acknowledgement so the last event is still read if the viewer then
  closes.
- Resumable file transfer. Chunks are uncompressed BLAKE3 pieces with a
  bitmap in a sidecar. `file_write` stays high-risk. The CLI `--put` path
  reads at most 32 MiB. zstd is not linked.
- Opus audio through rusty-opus after an Audio grant. `--tone` sends a
  synthetic packet and does not open a device. `cbdesk host --mic` and
  `--speaker`, and the matching view flags, open the default devices through
  cpal 0.15.3. A missing device is an error. This tree does not record that
  a person heard the stream. The jitter buffer waits for 40 ms and drops
  audio older than 120 ms.
- An operator-configured relay. Forbidden suffixes and n0 defaults still
  fail closed. The default endpoint still has an empty relay list. With a
  relay set, HTTPS net-report probes run only against that relay so a home
  relay can be chosen. A loopback relay test carries one probe with UDP
  off. A live symmetric NAT is still unmeasured. This project does not run a relay.
- Workspace layout and the first crates: protocol codec, identity, discovery
  records, in-memory transport, host session policy, and test doubles.
- Direct QUIC endpoint on iroh 1.4.1 with n0 relays and n0 discovery left
  uninstalled. Rust is pinned to 1.91.0, the `rust-version` of that release.
  T-NET-001 is a loopback echo whose resolver must not be asked for a name.
- A verified invite ticket dials its direct addresses. Relay URLs on the
  ticket are not contacted. The host grants that connection only after
  consent. An unanswered approval closes at 30 seconds with
  `TIMEOUT_USER_RESPONSE`. An echo probe is `PROTOCOL_VIOLATION` and does
  not grant a session.
- LAN announce and browse on the tested mDNS TXT records and the `CBDS`
  broadcast body. Public networks and a disabled user send nothing.
- A Flutter desktop shell in `app/desktop` (Flutter 3.47.7,
  flutter_rust_bridge 2.12.0). Home shows the device id and the short code.
  The other screens are connect, consent, a remote view, a file panel, an
  address book, and settings, in Turkish and English. Consent shows the
  fraud warning, waits on a first contact, and requires an explicit opt-in
  for a high-risk permission. Reject starts focused. The session indicator
  is an edge and a disconnect panel painted above the pages, and it has no
  off switch. The remote view is the decoded frame. A desktop host sends
  the operating-system capture when no picture is queued. The file panel
  sends bytes over QUIC. The microphone switch opens the default device.
  The two short-code words confirm pairing and do not locate a computer. A
  ticket is still only a locator. Widget tests use a fake desk and do not
  load the native library.
- GitHub Actions runs rustfmt, cargo-deny, RustSec audit, clippy, and tests
  on Linux, Windows, and macOS, and runs Flutter analyze and widget tests on
  those three. Ubuntu builds the Linux debug bundle and an arm64 Android
  debug APK. Windows and macOS build their debug bundles and check that
  `libcbdesk_ffi` is linked.
- ADRs 001–015, threat-model draft, and contribution documents.
- Turkish and English message catalogs for the errors and consent strings that
  exist so far.
- One unidirectional QUIC stream per video frame, opened only after
  `authorize(View)`. A side stream before the grant is `PROTOCOL_VIOLATION`.
  An input event uses its own unidirectional stream and is accepted only
  after `authorize(Input)`.
- Software AV1 (rav1e, assembler and git tooling off; rav1d-safe under
  AGPL-3.0-only) for one grayscale still.
- Software VP8 through oxideav-vp8 0.2.7 (MIT, default features off, no
  system libvpx). Negotiation prefers VP8 when both peers offer it. AV1 is
  the still-picture fallback.
- Operating-system screen capture through xcap 0.9.5, and keyboard/pointer
  injection through enigo after an Input grant. Tests record input events
  and do not move the pointer.
- `cbdesk host` streams frames until the viewer closes (`--frames 0`).
  `cbdesk view --window` shows them with winit and softbuffer and forwards
  the pointer and keyboard only after an Input grant. `--ppm` still writes
  the first frame. `--screen` captures the display. The ticket is still
  only a locator.
- Plain-text clipboard sync on a bidirectional stream after a Clipboard
  grant, capped at 60 KiB. Files and images are not this message.
- The device seed is loaded from the OS keychain (Secret Service, Keychain,
  or Credential Manager). A store that does not answer yields an in-memory
  identity for that process. The seed is not written to a file.
- LAN announce and browse can join `224.0.0.251:5353` when that port can be
  shared. A busy 5353 keeps sending to the group and listens on an ephemeral
  port. `--lan` is the operator opting into a private profile.
- ADR-015 keeps the clean-room core. The M0 fork exit stays unmet on purpose.

This tree is not derived from RustDesk. See ADR-002 and ADR-015.

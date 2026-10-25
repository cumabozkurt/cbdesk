# Owner and absent work

This page names work that is still open. The checkboxes in
`docs/MASTER_PLAN.md` stay unchecked. The ledger is
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md).
What the tests exercise is on the [status](status.md) page.

## Owner

A person, an account, or a machine outside this repository has to do these.
The tree does not contain a completed copy of any of them.

- GitHub organization settings, branch protection, CODEOWNERS, the GitHub
  DCO application, and Discussions. CI runs a local DCO script.
- The trademark search and a domain. `docs/brand/` records that the search
  was not done.
- An independent security audit. None has been commissioned.
- A notice to antivirus vendors. None has been sent.
- Signing keys, a signed installer, an Apple Developer account, and
  TestFlight. macOS CI type-checks the controller crate for the iOS device
  target. There is no signed iOS build.
- A Flathub submission. The Flatpak template id is `org.cbdesk.CBDesk`. It
  is not listed on Flathub.
- A person accepting the Android system capture dialog on a phone. CI
  builds an arm64 debug APK. That dialog was not accepted here.
- A second machine hashing a release binary.
  `tools/release/reproducible.sh` records `SOURCE_DATE_EPOCH`, `rustc -vV`,
  and the `Cargo.lock` digest.

## Absent

This tree does not contain these. A green CI run does not add them.

- The plan's M0 exit, a fork that builds on three desktops. This repository
  started from empty crates. ADR-015 keeps capture, the codec, and input in
  this core.
- A 30 frame-per-second stream and a glass-to-glass budget. One machine
  encoded one solid 1920×1080 software keyframe in 198 ms. Four parallel
  strip keyframes of that same solid frame took 52 ms. A quarter-size
  preview took 12 ms. `cbdesk host --strips` above 1 can send those bands
  on a VP8 session. The default stays on the damage encoder. The desk host
  does not take the flag. Each band is its own keyframe, not the
  inter-frame path. A detailed desktop was not measured. No successful
  hardware encode was recorded.
- A measured live symmetric NAT. A loopback path stays a loopback path.
  Netlab profiles do not create network namespaces.
- Blinded DHT. A blinded publish fails closed. ADR-007 stays Proposed.
- A public relay and a published image. `cbdesk-relayd` binds loopback
  only, prints a self-signed `https://` URL, and a test echoes one probe
  with UDP off. That is not a symmetric-NAT measurement.
  `packaging/relayd/Dockerfile` was not built. n0 defaults stay off.
- Quick support. The session policy bans it.
- The live `cbdesk host` holding the X11 grab, the privacy window, the
  shell, or the tunnel. Those functions exist and are tested on their own.
  Windows calls `BlockInput` and macOS creates a session event tap. A
  refused call or a null tap does not block input. This workspace did not
  record a successful grab. The cover window is still X11. The live host
  does not call them.
- A second physical monitor, and a monitor index on the desk session.
  `cbdesk host --monitor N` selects an index from `list_monitors` for that
  call. Omitting it keeps the primary display. The index is not a stable
  hardware id. An index past the list is refused. The desk session still
  calls `capture_picture`. This workspace did not attach a second display.
- Pasting a list of files. Clipboard text is implemented. File paste is
  absent.
- ConPTY, a signed virtual display, and a Windows indirect-display driver.
  Linux can start Xvfb from the host-operations library. The live host does
  not call that either.
- cargo-fuzz, libFuzzer, OSS-Fuzz, and Criterion. They are not wired and
  were not run.
- A 72-hour soak, a Linux-to-Windows session, a usability round with a
  person, a beta channel, and version 1.0.0. The version is 0.0.0.
- A Wayland ScreenCast run. The manual plan is
  `docs/dev/wayland-screencast.md`. CI did not run it.
- A confirmed listening test of the microphone path. A missing microphone
  is an error.

## Still ignored

RUSTSEC-2026-0387 stays ignored in `deny.toml` and `.cargo/audit.toml`
while `opaque-ke` 4.0.1 depends on `crypto-bigint` 0.5.5. Remove that
ignore in the commit that moves to a stable `opaque-ke` whose tree no
longer contains 0.5.5. Confirm with
`cargo tree -i crypto-bigint -p cbdesk-crypto` before deleting it.

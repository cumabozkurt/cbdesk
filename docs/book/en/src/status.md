# Status and limits

The plan checkboxes in `docs/MASTER_PLAN.md` are still unchecked. The
ledger that maps them is
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md).
This page is the short version for someone reading the guide. Work that
stays with the owner, and work that is absent, is listed on the
[limits](limits.md) page.

## What the tests actually exercise

- After consent, `cbdesk host` and `cbdesk view` can open a session on the
  local network. The picture is color. The synthetic source is a gray
  barcode.
- The Flutter shell shows the device id, connect, consent, an indicator
  with no off switch, a decoded frame, file bytes, an address book, and
  settings, in Turkish and English. Widget tests use a fake desk.
- A ticket dial, a loopback QUIC echo, clipboard text, one file put, and a
  synthetic Opus tone are covered by tests. A missing microphone is an
  error and does not pretend the audio was heard.
- Linux CI runs the X11 capture path under Xvfb. A private Xvfb also
  covers the input grab and the black window in the host-operations
  library.
- CI builds the debug shell on Ubuntu, Windows, and macOS, and an arm64
  Android debug APK. macOS CI type-checks the controller crate for the
  iOS device target.

## What was not tried

- No person has clicked through Android on a phone, and the system capture
  dialog was not accepted. There is no signed iOS build.
- Wayland ScreenCast is compiled and was not run in CI.
- A live symmetric NAT was not measured. A loopback path is not a hole
  punch. Netlab profiles do not create network namespaces.
- Nobody has confirmed they heard the microphone. A 1080p software
  keyframe on one machine took 201 ms. A later solid-frame run took 198 ms
  as one keyframe, 52 ms as four parallel strips, and 12 ms at quarter
  size. `--strips` above 1 can send those bands on a VP8 session. The
  default does not. That is not 30 frames per second. No successful
  hardware encode was recorded.
- The live host does not hold the X11 grab, the privacy window, the
  shell, or the tunnel, even though those functions exist and are tested
  on their own.
- No Linux-to-Windows session was run. No 72-hour soak. No second machine
  compared binaries. No external security audit. No signed package and no
  store listing.
- Tag `v0.1.0-alpha.1` is an unsigned alpha on GitHub. This workspace did
  not pack those archives, and this page does not rebuild them. The tag is
  not version 1.0.0.
- GitHub organization settings, branch protection, the trademark search,
  and signing keys stay with the owner.

M0's exit in the plan is a fork that builds on three desktops. This
repository is not that fork. A green CI run is not that exit.

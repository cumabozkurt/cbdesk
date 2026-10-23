# Building

## Requirements

- Rust 1.91.0 and the `rustfmt` and `clippy` components. `rust-toolchain.toml`
  selects them. ADR-006 records why the pin is 1.91 and not a newer compiler:
  that is the `rust-version` of iroh 1.4.1.
- A C toolchain (gcc, clang, or MSVC) is required. iroh's `tls-ring` feature
  compiles `ring`. BLAKE3 in this workspace still uses the `pure` feature.
- Flutter 3.47.7 (Dart 3.13.5) for the desktop shell in `app/desktop`. CI
  pins that version. The shell links the Rust core, so Rust 1.91.0 is still
  required. Linux also needs `cmake`, `ninja-build`, `libgtk-3-dev`,
  `clang`, `libstdc++-14-dev`, `pkg-config`, `libasound2-dev`, and the
  capture libraries listed below. Clang links the C++ standard library from
  that GCC 14 package.

## Commands

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Desktop shell, from `app/desktop`:

```sh
flutter config --no-analytics
flutter pub get
flutter analyze
flutter test
flutter build linux --debug
```

`flutter test` uses a fake desk and does not load `libcbdesk_ffi`. The debug
bundle does. A desktop host encodes `capture_picture` when no picture is
queued. The 48×16 motion pair is the queued test picture.

Windows and macOS debug bundles:

```sh
flutter build windows --debug
flutter config --no-enable-swift-package-manager
flutter build macos --debug
```

The macOS plugin is a CocoaPods podspec. Flutter 3.47's Swift package
generator left that plugin's dependencies empty, so the macOS build turns
Swift Package Manager off and links `libcbdesk_ffi.a` with `-force_load`.

Android, after the SDK and NDK 28.2.13676358 are installed:

```sh
rustup target add aarch64-linux-android
flutter build apk --debug --target-platform android-arm64
```

The APK is the same shell. Screen pixels are read only after the system
capture dialog returns acceptance. The longer side is scaled to at most
960 before `submit_frame`. A denial does not create a virtual display.

`cargo test` includes an Argon2id vector at the backup parameters (64 MiB,
3 passes). That test takes a fraction of a second on a desktop and longer on
a small CI runner. It is the parameter lock for identity backup, which is not
wired to a file format yet.

The QUIC echo binds two loopback sockets. The ticket dial tests do the same,
and they also name `192.0.2.1` so a higher-ranked address is started first.
That address is not expected to answer. The 30 second approval wait is locked
with a paused clock; the loopback handshake fires the same timeout path
without sleeping 30 wall-clock seconds. The LAN tests send the mDNS and
broadcast encodings between sockets. One of them joins `224.0.0.251:5353`
when that port is free. A public profile still sends nothing.

Linux screen capture, the viewer window, and input compile against system
libraries. Ubuntu CI installs `pkg-config`, `clang`, `libclang-dev`,
`libxcb1-dev`, `libxrandr-dev`, `libx11-dev`, `x11-utils`, `libdbus-1-dev`,
`libpipewire-0.3-dev`, `libwayland-dev`, `libxkbcommon-dev`,
`libxkbcommon-x11-dev`, `libegl1-mesa-dev`, `libgbm-dev`, `libdrm-dev`,
`libasound2-dev`, and `xvfb`. The Flutter Linux job also installs `cmake`,
`ninja-build`, `libgtk-3-dev`, and `libstdc++-14-dev`. The test step starts Xvfb on `:99` and sets `DISPLAY` and
`CBDESK_REQUIRE_DISPLAY=1`, so a missing X11 frame or a viewer window that
does not open fails that job. Windows and macOS compile the same capture and
window crates. Their jobs leave `CBDESK_REQUIRE_DISPLAY` unset, so the window
smoke test returns without opening a display, and a missing display or a
denied permission is `Unavailable`. The synthetic, codec, and consent tests
still run. The Wayland ScreenCast path is selected when `WAYLAND_DISPLAY` is
set. CI does not set that variable and does not open a portal dialog. Input
tests do not call `inject`.

The LAN unit test joins `224.0.0.251:5353` when the port can be shared with
another socket. A busy 5353 falls back to an ephemeral member and the test
checks that member on loopback. A public profile still sends nothing. The
CLI browse test and that unit test take turns on a localhost TCP mutex so
they do not share port 5353 at the same time.

## CLI

```sh
cbdesk host --bind 0.0.0.0:22116 --synthetic --approve view,input --lan
cbdesk view --browse --window
```

`--screen` captures the display instead of the barcode. The live picture
keeps that capture's even width and height. `--approve` is explicit consent
and still waits three seconds on a first contact. Omitting it rejects
immediately. `--frames N` stops after N frames; `0`, the default, streams
until the viewer closes. `--pointer X,Y` and `--key NAME` send one event
each after the first frame, and only if the host granted `input`. The
window uses the same mapping for later pointer and key events. `--put FILE`
uploads that file after `file_write`. `--tone` and `--audio` exchange one
synthetic Opus stream after `audio` and do not open a device. `--mic` opens
the default microphone and `--speaker` opens the default speaker. A missing
device is an error. `--tone` and `--mic` cannot both be set. `--relay URL`
may be repeated. The
ticket printed by the host is a locator. `--browse` does not use it.
`--lan` does not read the OS network category; the library still sends
nothing when a caller passes a public profile.

## Pins

Dependencies are exact versions. Do not change a version to a range. The
direct dependencies on `base64ct`, `hashbrown`, `indexmap`, `zeroize_derive`,
`tempfile`, and `toml_edit` keep transitive crates on releases this tree has
already compiled. iroh brings its own copy of newer majors where the crates
do not unify (`ed25519-dalek` 3, `getrandom` 0.4).

## What will not build

Installers have no code in the workspace. iOS has no shell. Some plan
directories still contain only a README and are not Cargo workspace
members. Plain-text clipboard sync is in the CLI and the desk session, not
in `cbdesk-clipboard`. File bytes live in `cbdesk-fs` and chunks are
uncompressed. Opus and the cpal devices live in `cbdesk-audio`. The Flutter
microphone switch opens the default device. A missing device does not end
the desk session. The macOS plugin file is a CocoaPods podspec. Flutter
3.47's generated Swift package did not list it, so the macOS build turns
Swift Package Manager off.

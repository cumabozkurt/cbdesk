# Install and download

Version 0.0.0 in this tree is not a release. This site does not host an
installer, a package, or a store listing. This commit does not publish a
file.

## Download

GitHub tag `v0.1.0-alpha.1` is titled `v0.1.0-alpha.1 unsigned alpha` and
attaches the files below. This site does not host them. This page does not
rebuild them. They are an unsigned alpha. They are not version 1.0.0 and
not the plan's alpha exit. Windows SmartScreen and macOS Gatekeeper can
warn or block them. There is no notarization, no Apple Developer account,
and no Authenticode certificate in the build. No real-device test and no
performance test of these binaries was recorded. A 30 frame-per-second
stream was not measured. The crate version is still 0.0.0.

The release page is
<https://github.com/cumabozkurt/cbdesk/releases/tag/v0.1.0-alpha.1>.

| File | What it is |
|---|---|
| [Windows desktop zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-windows-x64.zip) | Flutter bundle, x64 |
| [Windows CLI zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-x86_64-pc-windows-msvc.zip) | `cbdesk.exe` |
| [Linux desktop tar.gz](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-linux-x64.tar.gz) | Flutter bundle from `ubuntu-24.04` |
| [Linux CLI tar.gz](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz) | Links `ubuntu-24.04` system libraries |
| [Linux CLI deb](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk_0.1.0-alpha.1_amd64.deb) | Unsigned. Not the Flutter app |
| [macOS arm64 desktop zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-macos-arm64.zip) | Apple Silicon only. Not notarized |
| [macOS arm64 CLI](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-aarch64-apple-darwin.tar.gz) | Apple Silicon only. Not notarized |
| [Android arm64 debug APK](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-android-arm64-debug.apk) | Not installed on a phone |
| [SHA256SUMS](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/SHA256SUMS) | Digests, not a signature |

`sbom.spdx.json.sign-note` on that tag reads `unsigned: CBDESK_SIGN_KEY is unset`.

The workflow is written to upload:

- CLI archives from `cargo build --release --locked`. Linux is a `.tar.gz`
  for `x86_64-unknown-linux-gnu`, built on `ubuntu-24.04`, and it links
  system libraries from that image. Windows is a `.zip` with `cbdesk.exe`
  for `x86_64-pc-windows-msvc`. macOS is a `.tar.gz` for
  `aarch64-apple-darwin` from `macos-14`. That macOS file is Apple Silicon
  only. It is not a universal binary and it is not notarized.
- Flutter desktop bundles: a Linux release bundle `.tar.gz`, a Windows
  release `.zip`, and a macOS release `.app` `.zip`. The macOS app is
  arm64 only. It is not a universal binary and it is not notarized.
  Flutter 3.47 has no `--no-codesign` flag. The workflow turns code
  signing off (`CODE_SIGNING_ALLOWED=NO`). If a platform job fails, the
  release still receives the archives that were built and names the
  missing one.
- An arm64 Android debug APK. It is not a release-signed package. It was
  not installed on a phone, and the system capture dialog was not accepted.
- The unsigned deb and an SPDX SBOM. `CBDESK_SIGN_KEY` is unset, so the
  checksum note says unsigned.
- `SHA256SUMS`. That file is a list of digests. It is not a signature.

No MSI, DMG, rpm, AppImage, or Flatpak has been signed or published.
The repository still contains those templates. The Flatpak manifest id is
`org.cbdesk.CBDesk`. It has not been submitted to Flathub. The Windows
installer source has not been compiled. The DMG and AppImage scripts were
not run. There is no TestFlight build and no Apple Developer account in
this tree.

If someone phoned you and told you to install remote-access software,
refuse. Banks, government agencies, and support companies do not ask for
that.

## Building from source

A developer build needs Rust 1.91.0. The desktop shell needs Flutter
3.47.7. The steps are in the repository file
[docs/dev/building.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/building.md).
That build is for people working on the code. It is not a supported
install for someone who was asked to "just download remote access."

Checksums in the release scripts stay unsigned unless `CBDESK_SIGN_KEY` is
set outside the repository. No private key is stored in git.

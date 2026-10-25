<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Deployment notes

No installer was signed. No store listing was published. The files below
are templates. A command that was not run is named as such.

## Packages

`packaging/deb/build.sh` builds an unsigned deb of the release `cbdesk`
binary. `CBDESK_VERSION` defaults to `0.0.0`. `CBDESK_REUSE_BINARY=1` skips
a second `cargo build` when `target/release/cbdesk` is already there. It
was not run in this pass. `packaging/appimage/build.sh`
exits if `linuxdeploy` is missing and does not download it. It was not run.
`packaging/macos/make-dmg.sh` uses `hdiutil` and does not notarize. It
exits on Linux and was not run. `packaging/windows/cbdesk.wxs` is a WiX v4
source with no certificate. It was not compiled, so it is not an MSI yet.
`packaging/flatpak/org.cbdesk.CBDesk.yml` uses the package id
`org.cbdesk.CBDesk`. It points at this tree and has not been submitted to
Flathub.

`.github/workflows/release.yml` runs on a `v*` tag and on
`workflow_dispatch` when the dispatcher names a tag. It builds unsigned
CLI archives on Linux, Windows, and macOS (`cargo build --release
--locked`), Flutter desktop bundles, an arm64 debug Android APK, the
unsigned deb, and an SPDX SBOM. The publish job creates a GitHub
prerelease and uploads the files with `SHA256SUMS`. That job is the only
one with `contents: write`. The files are not signed. SmartScreen and
Gatekeeper can warn or block them. The workflow does not run on pull
requests. It was not run in this workspace, and no file was published.
The deb version string follows the tag with the leading `v` removed.
The crate version stays `0.0.0`.

Windows pkg alternatives, rpm, winget, Homebrew, AUR, F-Droid, and a Docker
image of `cbdesk-relayd` are not published. This repository does not ship a
relay daemon. A package registry, a TUF root, and an update URL are not
configured. ADR-013 stays the reason: no root key is checked into git, and
no updater runs. A GitHub prerelease is only the unsigned file upload
above.

## Checksums and build inputs

`tools/release/sbom.py --locked --check` was run here and listed 813
packages. The document is SPDX 2.3 JSON from `cargo metadata`.
`filesAnalyzed` is false. A crate with no license field becomes
`NOASSERTION`. `tools/release/sign.sh` writes a sha256 file. When
`CBDESK_SIGN_KEY` is unset it writes a note that starts with `unsigned` and
exits 0. No private key is stored in git. `tools/release/reproducible.sh`
prints `SOURCE_DATE_EPOCH`, `rustc -vV`, and the sha256 of `Cargo.lock`.
It does not build twice and does not claim the binary matches another
machine.

## Organization rollout

Group Policy, Intune, Jamf, and Ansible would install a signed package and
would not disable the session indicator. Those tools were not pointed at a
package, because there is no signed package. An organization certificate
format is not parsed. ADR-011 stays Proposed.

## iOS and Android

`cbdesk-controller` is the view-side crate macOS CI can type-check for
`aarch64-apple-ios` (`tools/ios/check-controller.sh`). A signed TestFlight
build needs an Apple Developer membership, a provisioning profile, and a
device. None of those are in this tree. The script was not run against the
iphoneos SDK here because this machine is not macOS.

`tools/android/device-check.sh` uses `adb` when a device is attached.
`CBDESK_ANDROID_DEVICE=1` makes a missing device a failure. CI does not set
that. This machine has no `adb` and no phone. The script names
`org.cbdesk.cbdesk/.MainActivity` and does not accept the system capture
dialog.

## Virtual display

`open_virtual_display` starts `Xvfb` on Linux when that binary is installed
and stops it when the value is dropped. Windows indirect-display signing
has not started. macOS has no virtual-display entitlement here. The live
capture path does not call `open_virtual_display`.

## Audit and soak

An independent security audit has not been commissioned. A 72-hour soak
was not run. Antivirus vendors were not notified. A bit-for-bit rebuild
was not compared. Those stay with the owner or with another machine.

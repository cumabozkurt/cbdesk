# Install and download

There is nothing to download. Version 0.0.0 is not a release. This site
does not host an installer, a package repository, or a store listing.

No MSI, DMG, deb, rpm, AppImage, or Flatpak has been signed or published.
The repository contains unsigned templates only. The Flatpak manifest id is
`org.cbdesk.CBDesk`. It has not been submitted to Flathub. The Windows
installer source has not been compiled. The DMG and AppImage scripts were
not run. A debug Android APK can be built in CI and was not installed on a
phone. The iOS controller crate can be type-checked on macOS. There is no
TestFlight build and no Apple Developer account in this tree.

## Building from source

A developer build needs Rust 1.91.0. The desktop shell needs Flutter
3.47.7. The steps are in the repository file
[docs/dev/building.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/building.md).
That build is for people working on the code. It is not a supported
install for someone who was asked to "just download remote access."

Checksums in the release scripts stay unsigned unless `CBDESK_SIGN_KEY` is
set outside the repository. No private key is stored in git.

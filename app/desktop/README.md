# CBDesk desktop shell

Flutter 3.47.7 shell over the Rust core through flutter_rust_bridge 2.12.0.
The package id is `org.cbdesk.CBDesk` on desktop and `org.cbdesk.cbdesk` on
Android. The binary name is `cbdesk`.

Screens: home (device id and short code), connect, consent, remote view,
file panel, address book, and settings. Turkish and English.

The session indicator is an edge and a disconnect panel. It has no off
switch. The two short-code words confirm pairing with `spake2` 0.4.0. They
do not locate a computer, and the host still approves. A ticket is a
locator. Connect accepts the ticket and, in the second field, the code.

The remote view paints the frame the core decoded. A desktop host sends
the operating-system capture when no picture is queued. A click and Enter
are forwarded after an Input grant. Clipboard text is sent after a
Clipboard grant. The microphone switch opens the default device after an
Audio grant. A missing device does not end the session. This tree does not
record that a person heard the audio.

The file panel jails the name and sends the bytes over QUIC. Android shares
the screen only after the system capture dialog. The longer side is at most
960. A denial does not start a virtual display.

```sh
flutter pub get
flutter analyze
flutter test
flutter build linux --debug
flutter build apk --debug --target-platform android-arm64
```

`flutter test` uses a fake desk and does not load `libcbdesk_ffi`.
macOS builds need `flutter config --no-enable-swift-package-manager` so the
CocoaPods podspec links `libcbdesk_ffi.a`.

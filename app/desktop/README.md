# CBDesk desktop shell

Flutter 3.47.7 shell over the Rust core through flutter_rust_bridge 2.12.0.
The package id is `org.cbdesk.CBDesk`. The binary name is `cbdesk`.

Screens: home (device id and short code), connect, consent, remote view,
file panel, address book, and settings. Turkish and English.

The session indicator is an edge and a disconnect panel. It has no off
switch. A short code does not authenticate. A ticket is a locator. The
remote view shows the core's decoded 48×16 motion frame after consent. It
is not an operating-system capture. The file panel stages a name in the
jail. Settings show audio device names and do not start a stream.
`cbdesk --screen`, `--put`, `--mic`, and `--speaker` remain the CLI paths.

```sh
flutter pub get
flutter analyze
flutter test
flutter build linux --debug
```

`flutter test` uses a fake desk and does not load `libcbdesk_ffi`.

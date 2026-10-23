<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# ADR-003: Flutter for the product UI

- Status: Accepted
- Date: 2026-10-23

## Context

The plan needs one UI toolkit for desktop and, later, mobile. The core stays
in Rust.

## Options

- A native toolkit per platform. Best platform fit, six UI code paths.
- A Rust UI toolkit. One language, a smaller ecosystem for mobile.
- Flutter, with a narrow FFI crate (`cbdesk-ffi`) in front of the Rust core.

## Decision

Flutter is the product UI. It is not started. `app/i18n` holds the catalogs
the Rust error codes already require, so translations are not blocked on the
widget tree. No Dart package is in this milestone, and CI does not run
`flutter analyze`.

## Consequences

The CLI still calls the session core directly. That path must not grow a
hidden session. The shell, the version pins, and what CI runs are in the
implementation note.

## Implementation note (2026-10-23)

The decision text above is unchanged. The shell now exists at `app/desktop`.
Flutter is pinned to 3.47.7. The bridge is flutter_rust_bridge 2.12.0 over
`cbdesk-ffi`. The hand-written API forbids unsafe; the generated glue is
the unsafe file. Screens in this tree are home (device id and short code),
connect, consent, remote view, file panel, address book, and settings, in
Turkish and English. The session indicator is painted above the pages and
has no off switch. The remote view shows the core's decoded 48×16 motion
frame after consent. It does not show an operating-system capture. The file
panel stages a name; `cbdesk view --put` is still the QUIC transfer. Settings
show microphone and speaker names and do not start those devices. A short
code does not authenticate. Widget tests use a fake desk. CI runs
`flutter analyze` and `flutter test` on Linux, Windows, and macOS, and
`flutter build linux --debug` on Ubuntu.

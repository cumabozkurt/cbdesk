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

FFI and the Flutter version pin are future work. A temporary CLI or test
harness may call `HostSession` directly before the Flutter shell exists.
That harness must not grow a hidden session.

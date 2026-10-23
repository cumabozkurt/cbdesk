// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Test doubles for later media and network tests.
//!
//! [`SyntheticCapturer`] does not touch a display. [`FakeDht`] does not send
//! packets. Nothing in this crate enables a hidden session.

mod capture;
mod dht;
mod error;
mod inject;

pub use capture::{read_barcode, Frame, SyntheticCapturer, BARCODE_WIDTH};
pub use dht::FakeDht;
pub use error::TestkitError;
pub use inject::RecordingInjector;

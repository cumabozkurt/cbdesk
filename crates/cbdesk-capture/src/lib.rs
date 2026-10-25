// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Screen sources for one session.
//!
//! [`SyntheticCapturer`] paints a barcode and does not touch a display.
//! [`capture_screen`] asks the operating system. On Linux that is X11, or the
//! Wayland ScreenCast portal plus PipeWire when `WAYLAND_DISPLAY` is set.
//! Windows and macOS use the same capture crate. Nothing here runs before a
//! person has granted `View`. This crate does not know about grants; the
//! caller does.

mod barcode;
mod error;
mod fit;
mod picture;
mod screen;
mod virtual_display;

pub use barcode::{read_barcode, Frame, SyntheticCapturer, BARCODE_WIDTH};
pub use error::CaptureError;
pub use fit::fit_even;
pub use picture::{crop_even, from_rgba, yuv_to_rgb, Picture};
pub use screen::{capture_picture, capture_screen};
pub use virtual_display::{open_virtual_display, VirtualDisplay};

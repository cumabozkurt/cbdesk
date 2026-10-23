// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Host and viewer for one consented picture.
//!
//! `cbdesk host` prints an invite ticket and, when `--approve` names `view`,
//! waits out the first-contact delay and sends one AV1 frame. Without
//! `--approve` the host rejects the prompt. `cbdesk view` dials the ticket,
//! decodes the frame, and writes a PPM file. A ticket is still only a locator.
//! `--screen` captures the operating system. `--synthetic` paints a barcode
//! and never touches a display. Input reaches the operating system only for
//! `--screen` after the grant includes `input`.

mod args;
mod error;
mod hex;
mod host;
mod ppm;
mod view;

pub use args::{parse, Command, HostArgs, ViewArgs};
pub use error::CliError;
pub use host::serve;
pub use view::view_remote;

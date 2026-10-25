// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Host and viewer for a consented picture stream.
//!
//! `cbdesk host` prints an invite ticket and, when `--approve` names `view`,
//! waits out the first-contact delay and streams frames until the viewer
//! closes. `--frames N` stops after N frames. Without `--approve` the host
//! rejects the prompt. `cbdesk view` dials the ticket or browses the LAN.
//! `--ppm` writes the first frame. `--window` keeps showing frames until the
//! window closes. A ticket is still only a locator. `--screen` captures the
//! operating system. `--synthetic` paints a barcode and never touches a
//! display. Input reaches the operating system only for `--screen` after the
//! grant includes `input`. The window forwards the pointer and keyboard only
//! after that same grant.

mod args;
mod check;
mod clipboard;
mod error;
mod files;
mod hex;
mod host;
mod identity;
mod lan;
mod ppm;
mod view;
mod window;

pub use args::{parse, Command, HostArgs, ViewArgs};
pub use check::{run_net_check, NetCheckArgs};
pub use error::CliError;
pub use host::serve;
pub use view::{view_remote, view_window};

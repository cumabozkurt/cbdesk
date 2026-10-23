// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Keyboard and pointer injection.
//!
//! [`InputLog`] records events for tests. [`inject_os`] talks to the
//! operating system and is what a granted `Input` session calls. Callers
//! must already have passed `authorize(Input)`. This crate does not grant
//! that permission, and it has no hidden-injection mode.

mod error;
mod log;
mod os;

pub use error::InputError;
pub use log::InputLog;
pub use os::inject_os;

/// Enter.
pub const KEY_ENTER: u32 = 0xE000;
/// Escape.
pub const KEY_ESCAPE: u32 = 0xE001;
/// Backspace.
pub const KEY_BACKSPACE: u32 = 0xE002;
/// Tab.
pub const KEY_TAB: u32 = 0xE003;
/// Left arrow.
pub const KEY_LEFT: u32 = 0xE004;
/// Right arrow.
pub const KEY_RIGHT: u32 = 0xE005;
/// Up arrow.
pub const KEY_UP: u32 = 0xE006;
/// Down arrow.
pub const KEY_DOWN: u32 = 0xE007;

/// Shift.
pub const MOD_SHIFT: u32 = 1;
/// Control.
pub const MOD_CTRL: u32 = 2;
/// Alt.
pub const MOD_ALT: u32 = 4;
/// Meta or Command.
pub const MOD_META: u32 = 8;

/// Left button.
pub const BUTTON_LEFT: u32 = 1;
/// Right button.
pub const BUTTON_RIGHT: u32 = 2;
/// Middle button.
pub const BUTTON_MIDDLE: u32 = 4;

/// Something that can apply one input event.
pub trait Injector {
    /// Apply `event`. A log records it. The OS backend injects it.
    fn inject(&mut self, event: &cbdesk_proto::InputEvent) -> Result<(), InputError>;
}

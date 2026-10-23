// SPDX-License-Identifier: AGPL-3.0-or-later

//! Operating-system injection.
//!
//! [`connect`] opens the desktop connection and does not move the pointer.
//! [`OsInjector::inject`] is the call that does. Tests may connect when a
//! display exists. They must not call `inject`: that would move the real
//! cursor. A synthetic host uses [`crate::InputLog`] instead.

use cbdesk_proto::{InputBody, InputEvent, KeyEvent, PointerEvent};

use crate::error::InputError;
use crate::{
    Injector, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT, KEY_BACKSPACE, KEY_DOWN, KEY_ENTER,
    KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_TAB, KEY_UP, MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT,
};

/// Apply `event` on this machine.
///
/// Returns [`InputError::Unavailable`] when this build has no backend or the
/// desktop connection cannot be opened. The caller has to have granted Input
/// before it gets here.
pub fn inject_os(event: &InputEvent) -> Result<(), InputError> {
    let mut injector = connect()?;
    injector.inject(event)
}

/// Open a desktop connection without sending an event.
pub fn connect() -> Result<OsInjector, InputError> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        platform::connect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = event_unused();
        Err(InputError::Unavailable(
            "this operating system has no input backend".to_string(),
        ))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn event_unused() -> bool {
    false
}

/// A live desktop injector. Dropping it does not release keys.
pub struct OsInjector {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    inner: platform::Backend,
}

impl Injector for OsInjector {
    fn inject(&mut self, event: &InputEvent) -> Result<(), InputError> {
        #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
        {
            self.inner.apply(event)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            let _ = event;
            Err(InputError::Unavailable(
                "this operating system has no input backend".to_string(),
            ))
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
mod platform {
    use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

    use super::*;

    pub(super) struct Backend {
        enigo: Enigo,
        buttons: u32,
        modifiers: u32,
    }

    pub(super) fn connect() -> Result<OsInjector, InputError> {
        let settings = Settings {
            release_keys_when_dropped: false,
            open_prompt_to_get_permissions: false,
            ..Settings::default()
        };
        let enigo =
            Enigo::new(&settings).map_err(|err| InputError::Unavailable(err.to_string()))?;
        Ok(OsInjector {
            inner: Backend {
                enigo,
                buttons: 0,
                modifiers: 0,
            },
        })
    }

    impl Backend {
        pub(super) fn apply(&mut self, event: &InputEvent) -> Result<(), InputError> {
            match &event.body {
                InputBody::Key(key) => self.apply_key(key),
                InputBody::Pointer(pointer) => self.apply_pointer(pointer),
            }
        }

        fn apply_key(&mut self, event: &KeyEvent) -> Result<(), InputError> {
            self.sync_modifiers(event.modifiers)?;
            let key = map_key(event.keycode)?;
            let direction = if event.down {
                Direction::Press
            } else {
                Direction::Release
            };
            self.enigo
                .key(key, direction)
                .map_err(|err| InputError::Failed(err.to_string()))
        }

        fn apply_pointer(&mut self, event: &PointerEvent) -> Result<(), InputError> {
            self.enigo
                .move_mouse(event.x, event.y, Coordinate::Abs)
                .map_err(|err| InputError::Failed(err.to_string()))?;
            self.sync_buttons(event.buttons)?;
            if event.wheel != 0 {
                self.enigo
                    .scroll(event.wheel, Axis::Vertical)
                    .map_err(|err| InputError::Failed(err.to_string()))?;
            }
            Ok(())
        }

        fn sync_buttons(&mut self, next: u32) -> Result<(), InputError> {
            for (bit, button) in [
                (BUTTON_LEFT, Button::Left),
                (BUTTON_RIGHT, Button::Right),
                (BUTTON_MIDDLE, Button::Middle),
            ] {
                let was = self.buttons & bit != 0;
                let now = next & bit != 0;
                if was == now {
                    continue;
                }
                let direction = if now {
                    Direction::Press
                } else {
                    Direction::Release
                };
                self.enigo
                    .button(button, direction)
                    .map_err(|err| InputError::Failed(err.to_string()))?;
            }
            self.buttons = next;
            Ok(())
        }

        fn sync_modifiers(&mut self, next: u32) -> Result<(), InputError> {
            for (bit, key) in [
                (MOD_SHIFT, Key::Shift),
                (MOD_CTRL, Key::Control),
                (MOD_ALT, Key::Alt),
                (MOD_META, Key::Meta),
            ] {
                let was = self.modifiers & bit != 0;
                let now = next & bit != 0;
                if was == now {
                    continue;
                }
                let direction = if now {
                    Direction::Press
                } else {
                    Direction::Release
                };
                self.enigo
                    .key(key, direction)
                    .map_err(|err| InputError::Failed(err.to_string()))?;
            }
            self.modifiers = next;
            Ok(())
        }
    }

    fn map_key(code: u32) -> Result<Key, InputError> {
        let named = match code {
            KEY_ENTER => Some(Key::Return),
            KEY_ESCAPE => Some(Key::Escape),
            KEY_BACKSPACE => Some(Key::Backspace),
            KEY_TAB => Some(Key::Tab),
            KEY_LEFT => Some(Key::LeftArrow),
            KEY_RIGHT => Some(Key::RightArrow),
            KEY_UP => Some(Key::UpArrow),
            KEY_DOWN => Some(Key::DownArrow),
            _ => None,
        };
        if let Some(key) = named {
            return Ok(key);
        }
        match char::from_u32(code) {
            Some(ch) if ch.is_ascii_graphic() || ch == ' ' => Ok(Key::Unicode(ch)),
            _ => Err(InputError::UnsupportedKey(code)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_reports_unavailable_or_opens_without_injecting() {
        match connect() {
            Ok(_injector) => {}
            Err(InputError::Unavailable(_)) => {}
            Err(error) => panic!("connect failed unexpectedly: {error}"),
        }
    }
}

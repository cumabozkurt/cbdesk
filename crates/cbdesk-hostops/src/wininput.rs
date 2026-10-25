// SPDX-License-Identifier: AGPL-3.0-or-later

//! Windows `BlockInput`.
//!
//! The guard calls `BlockInput(TRUE)` and `Drop` calls `BlockInput(FALSE)`.
//! A service session often refuses the call. That refusal is
//! [`OsError::Unavailable`]. This module does not record that a desktop was
//! blocked. The privacy cover stays an X11 window.

use std::os::raw::c_int;

use cbdesk_session::{OsPermit, PermitKind};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::BlockInput;

use crate::OsError;

/// A process-wide input block. Drop releases it.
#[derive(Debug)]
pub struct InputGrab {
    held: bool,
}

impl Drop for InputGrab {
    fn drop(&mut self) {
        if self.held {
            // SAFETY: BlockInput(FALSE) is the documented release for a
            // previous BlockInput(TRUE) in this process. The argument is the
            // Win32 BOOL 0. Failure is ignored because Drop cannot report it.
            unsafe {
                let _ = BlockInput(0);
            }
            self.held = false;
        }
    }
}

/// No Windows privacy window is implemented.
#[derive(Debug)]
pub struct PrivacyCover;

/// Block keyboard and mouse for this session.
pub fn block_local_input(permit: OsPermit, _display: Option<&str>) -> Result<InputGrab, OsError> {
    if permit.kind() != PermitKind::BlockInput {
        return Err(OsError::WrongPermit);
    }
    // SAFETY: BlockInput is the Win32 call that blocks this session's input.
    // TRUE is the BOOL 1. A zero return means the session refused it, and
    // nothing was blocked. The guard's Drop passes 0.
    let accepted: c_int = unsafe { BlockInput(1) };
    if accepted == 0 {
        return Err(OsError::Unavailable(
            "BlockInput was refused; this session did not block input".to_string(),
        ));
    }
    Ok(InputGrab { held: true })
}

/// Privacy mode is still an X11 window.
pub fn cover_display(permit: OsPermit, _display: Option<&str>) -> Result<PrivacyCover, OsError> {
    let _ = permit.kind();
    Err(OsError::Unavailable(
        "privacy mode is an X11 window; Windows has no cover in this tree".to_string(),
    ))
}

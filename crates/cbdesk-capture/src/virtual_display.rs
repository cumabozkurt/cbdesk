// SPDX-License-Identifier: AGPL-3.0-or-later

//! A virtual display is not opened.
//!
//! The plan's headless host uses a Windows indirect display driver. That
//! driver is not in this tree, and its signing process has not been started.
//! [`open_virtual_display`] fails closed. Screen capture still uses a real
//! display when one is present.

use crate::CaptureError;

/// The only virtual-display state this build has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualDisplay {
    /// No virtual display was created.
    Unavailable,
}

/// Refuse to create a virtual display.
pub fn open_virtual_display() -> Result<VirtualDisplay, CaptureError> {
    Err(CaptureError::Unavailable(
        "virtual display is not implemented; Windows IDD signing has not started".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_virtual_display_is_unavailable() {
        assert!(open_virtual_display().is_err());
    }
}

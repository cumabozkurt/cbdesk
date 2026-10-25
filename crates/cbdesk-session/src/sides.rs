// SPDX-License-Identifier: AGPL-3.0-or-later

//! Switching which computer is the host.
//!
//! Both peers have to agree. This module records that agreement. It does not
//! move a capture or an input stream.

use crate::SessionError;

/// Consents collected so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideSwitch {
    host_agreed: bool,
    controller_agreed: bool,
}

impl SideSwitch {
    /// Neither side has agreed.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current host agrees to become the controller.
    pub fn host_agrees(mut self) -> Self {
        self.host_agreed = true;
        self
    }

    /// The current controller agrees to become the host.
    pub fn controller_agrees(mut self) -> Self {
        self.controller_agreed = true;
        self
    }
}

/// Accept the switch only when both sides agreed.
pub fn commit_side_switch(switch: SideSwitch) -> Result<(), SessionError> {
    if switch.host_agreed && switch.controller_agreed {
        Ok(())
    } else {
        Err(SessionError::SideSwitchNeedsBoth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_side_cannot_switch_alone() {
        assert!(commit_side_switch(SideSwitch::new().host_agrees()).is_err());
        commit_side_switch(SideSwitch::new().host_agrees().controller_agrees()).expect("both");
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Consent-gated session controls that do not touch the operating system.
//!
//! Privacy mode and block-input are recorded as requests. This build does not
//! blank a monitor and does not block the local keyboard. The session
//! indicator stays a separate constant: these functions cannot turn it off.
//! Recording is refused unless the grant includes it and the host was told.

use cbdesk_proto::Permission;

use crate::perms::PermissionSet;
use crate::SessionError;

/// What this build does to the operating system for a control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostEffect {
    /// The operating system was not changed.
    Unavailable,
}

/// Privacy mode does not blank the display in this build.
pub const PRIVACY_MODE_OS: HostEffect = HostEffect::Unavailable;

/// Block-input does not grab the local keyboard in this build.
pub const BLOCK_INPUT_OS: HostEffect = HostEffect::Unavailable;

/// Record that privacy mode was requested.
///
/// The returned effect is [`HostEffect::Unavailable`]. `indicator_on` must be
/// true. There is no argument that hides the indicator.
pub fn request_privacy_mode(
    grant: PermissionSet,
    indicator_on: bool,
) -> Result<HostEffect, SessionError> {
    require(grant, Permission::PrivacyMode, indicator_on)?;
    Ok(PRIVACY_MODE_OS)
}

/// Record that local input should be blocked.
///
/// The returned effect is [`HostEffect::Unavailable`].
pub fn request_block_input(
    grant: PermissionSet,
    indicator_on: bool,
) -> Result<HostEffect, SessionError> {
    require(grant, Permission::BlockInput, indicator_on)?;
    Ok(BLOCK_INPUT_OS)
}

/// Accept a recording request only when the host was notified.
///
/// This function does not write a file and does not encrypt one.
pub fn request_recording(
    grant: PermissionSet,
    indicator_on: bool,
    host_notified: bool,
) -> Result<(), SessionError> {
    require(grant, Permission::Record, indicator_on)?;
    if !host_notified {
        return Err(SessionError::RecordingNeedsNotice);
    }
    Ok(())
}

/// Accept a terminal channel only when Terminal was granted.
///
/// This does not spawn a shell or a PTY.
pub fn open_terminal(grant: PermissionSet, indicator_on: bool) -> Result<(), SessionError> {
    require(grant, Permission::Terminal, indicator_on)
}

/// Accept a TCP tunnel only when Tunnel was granted.
///
/// This does not bind a socket.
pub fn open_tunnel(grant: PermissionSet, indicator_on: bool) -> Result<(), SessionError> {
    require(grant, Permission::Tunnel, indicator_on)
}

fn require(
    grant: PermissionSet,
    permission: Permission,
    indicator_on: bool,
) -> Result<(), SessionError> {
    if !indicator_on {
        return Err(SessionError::MarkupNeedsIndicator);
    }
    if !grant.contains(permission) {
        return Err(SessionError::PermissionDenied);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn granted(permission: Permission) -> PermissionSet {
        let mut set = PermissionSet::empty();
        set.insert(permission);
        set
    }

    #[test]
    fn privacy_and_block_input_do_not_change_the_operating_system() {
        let effect = request_privacy_mode(granted(Permission::PrivacyMode), true).expect("privacy");
        assert_eq!(effect, HostEffect::Unavailable);
        let effect = request_block_input(granted(Permission::BlockInput), true).expect("block");
        assert_eq!(effect, HostEffect::Unavailable);
        assert!(request_privacy_mode(granted(Permission::PrivacyMode), false).is_err());
        assert!(request_recording(granted(Permission::Record), true, false).is_err());
        request_recording(granted(Permission::Record), true, true).expect("record");
        assert!(open_terminal(PermissionSet::empty(), true).is_err());
        open_terminal(granted(Permission::Terminal), true).expect("terminal");
        assert!(open_tunnel(granted(Permission::Tunnel), false).is_err());
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Consent gates for host controls.
//!
//! These functions do not touch the operating system. They return an
//! [`OsPermit`] that `cbdesk-hostops` consumes. The session indicator stays a
//! separate constant: there is no argument that turns it off. Recording is
//! refused unless the grant includes it and the host was told.

use cbdesk_proto::Permission;

use crate::perms::PermissionSet;
use crate::SessionError;

/// Which operating-system effect a permit allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermitKind {
    /// Blank the local display.
    Privacy,
    /// Grab the local keyboard and pointer.
    BlockInput,
    /// Open a shell.
    Terminal,
    /// Bind a loopback TCP forwarder.
    Tunnel,
    /// Tell the person at the host that recording was requested.
    Recording,
}

/// Proof that a grant and the indicator allow one effect.
///
/// The operating-system crate consumes this. Holding it does not change the
/// machine by itself.
#[derive(Debug)]
pub struct OsPermit {
    kind: PermitKind,
}

impl OsPermit {
    /// Which effect this permit allows.
    pub const fn kind(&self) -> PermitKind {
        self.kind
    }

    fn issue(kind: PermitKind) -> Self {
        Self { kind }
    }
}

/// Permit privacy mode when the grant and the indicator allow it.
pub fn request_privacy_mode(
    grant: PermissionSet,
    indicator_on: bool,
) -> Result<OsPermit, SessionError> {
    require(grant, Permission::PrivacyMode, indicator_on)?;
    Ok(OsPermit::issue(PermitKind::Privacy))
}

/// Permit a local input grab when the grant and the indicator allow it.
pub fn request_block_input(
    grant: PermissionSet,
    indicator_on: bool,
) -> Result<OsPermit, SessionError> {
    require(grant, Permission::BlockInput, indicator_on)?;
    Ok(OsPermit::issue(PermitKind::BlockInput))
}

/// Permit a recording notice when the host was told.
///
/// This function does not write a file and does not encrypt one.
pub fn request_recording(
    grant: PermissionSet,
    indicator_on: bool,
    host_notified: bool,
) -> Result<OsPermit, SessionError> {
    require(grant, Permission::Record, indicator_on)?;
    if !host_notified {
        return Err(SessionError::RecordingNeedsNotice);
    }
    Ok(OsPermit::issue(PermitKind::Recording))
}

/// Permit a terminal when Terminal was granted.
///
/// This does not spawn a shell.
pub fn open_terminal(grant: PermissionSet, indicator_on: bool) -> Result<OsPermit, SessionError> {
    require(grant, Permission::Terminal, indicator_on)?;
    Ok(OsPermit::issue(PermitKind::Terminal))
}

/// Permit a TCP tunnel when Tunnel was granted.
///
/// This does not bind a socket.
pub fn open_tunnel(grant: PermissionSet, indicator_on: bool) -> Result<OsPermit, SessionError> {
    require(grant, Permission::Tunnel, indicator_on)?;
    Ok(OsPermit::issue(PermitKind::Tunnel))
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
    fn permits_require_the_grant_and_the_indicator() {
        assert_eq!(
            request_privacy_mode(granted(Permission::PrivacyMode), true)
                .expect("privacy")
                .kind(),
            PermitKind::Privacy
        );
        assert_eq!(
            request_block_input(granted(Permission::BlockInput), true)
                .expect("block")
                .kind(),
            PermitKind::BlockInput
        );
        assert!(request_privacy_mode(granted(Permission::PrivacyMode), false).is_err());
        assert!(request_recording(granted(Permission::Record), true, false).is_err());
        assert_eq!(
            request_recording(granted(Permission::Record), true, true)
                .expect("record")
                .kind(),
            PermitKind::Recording
        );
        assert!(open_terminal(PermissionSet::empty(), true).is_err());
        assert_eq!(
            open_terminal(granted(Permission::Terminal), true)
                .expect("terminal")
                .kind(),
            PermitKind::Terminal
        );
        assert!(open_tunnel(granted(Permission::Tunnel), false).is_err());
        assert_eq!(
            open_tunnel(granted(Permission::Tunnel), true)
                .expect("tunnel")
                .kind(),
            PermitKind::Tunnel
        );
    }
}

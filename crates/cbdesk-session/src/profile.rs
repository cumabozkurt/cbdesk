// SPDX-License-Identifier: AGPL-3.0-or-later

//! Named permission sets.
//!
//! None of these include a hidden-session permission, because that permission
//! does not exist. High-risk permissions stay out of the preset. A file-write
//! or terminal grant still needs the explicit high-risk opt-in on first contact.

use cbdesk_proto::Permission;

use crate::perms::PermissionSet;

/// A preset the host can offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessProfile {
    /// View, input, clipboard, audio, and system info.
    Support,
    /// View only.
    ViewOnly,
    /// View and file read. File write is not included.
    Files,
    /// The same set as [`AccessProfile::Support`].
    ///
    /// "Full" here is the safe set, not every permission the protocol can name.
    Full,
}

/// Permissions in `profile`.
pub fn profile_permissions(profile: AccessProfile) -> PermissionSet {
    let mut set = PermissionSet::empty();
    let perms: &[Permission] = match profile {
        AccessProfile::ViewOnly => &[Permission::View],
        AccessProfile::Files => &[Permission::View, Permission::FileRead],
        AccessProfile::Support | AccessProfile::Full => &[
            Permission::View,
            Permission::Input,
            Permission::Clipboard,
            Permission::Audio,
            Permission::Sysinfo,
        ],
    };
    for perm in perms {
        set.insert(*perm);
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_leave_out_high_risk_permissions() {
        for profile in [
            AccessProfile::Support,
            AccessProfile::ViewOnly,
            AccessProfile::Files,
            AccessProfile::Full,
        ] {
            let set = profile_permissions(profile);
            assert!(!set.contains(Permission::Terminal));
            assert!(!set.contains(Permission::FileWrite));
            assert!(!set.contains(Permission::PrivacyMode));
            assert!(!set.contains(Permission::BlockInput));
            assert!(set.contains(Permission::View));
        }
        assert!(!profile_permissions(AccessProfile::ViewOnly).contains(Permission::Input));
        assert!(profile_permissions(AccessProfile::Files).contains(Permission::FileRead));
    }
}

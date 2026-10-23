// SPDX-License-Identifier: AGPL-3.0-or-later

//! Permission sets granted for one session.
//!
//! The unattended set is view, input, clipboard, audio, and system info.
//! High-risk permissions are not part of it.

use cbdesk_proto::Permission;

/// Bits for permissions that can actually be granted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PermissionSet(u32);

const SAFE: [Permission; 5] = [
    Permission::View,
    Permission::Input,
    Permission::Clipboard,
    Permission::Audio,
    Permission::Sysinfo,
];

impl PermissionSet {
    /// No permissions.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// What an unattended grant is allowed to include.
    pub fn safe_unattended() -> Self {
        let mut set = Self::empty();
        for perm in SAFE {
            set.insert(perm);
        }
        set
    }

    /// Insert `perm` when it has a grant bit.
    pub fn insert(&mut self, perm: Permission) {
        if let Some(bit) = perm.bit() {
            self.0 |= 1_u32 << bit;
        }
    }

    /// Whether `perm` was granted.
    pub fn contains(self, perm: Permission) -> bool {
        perm.bit().is_some_and(|bit| self.0 & (1_u32 << bit) != 0)
    }

    /// Granted permissions, in wire order.
    pub fn granted(self) -> Vec<Permission> {
        let mut out = Vec::new();
        for value in 1..=17 {
            let perm = Permission::from_i32(value);
            if self.contains(perm) {
                out.push(perm);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_set_excludes_high_risk_permissions() {
        let set = PermissionSet::safe_unattended();
        for perm in SAFE {
            assert!(set.contains(perm));
            assert!(!perm.is_high_risk());
        }
        assert!(!set.contains(Permission::FileWrite));
        assert!(!set.contains(Permission::Terminal));
        assert!(!set.contains(Permission::BlockInput));
        assert!(!set.contains(Permission::PrivacyMode));
    }
}

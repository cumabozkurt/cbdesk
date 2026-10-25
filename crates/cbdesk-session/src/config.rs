// SPDX-License-Identifier: AGPL-3.0-or-later

//! Security settings.
//!
//! Unknown keys are rejected, so a `stealth` flag cannot be smuggled in.
//! Quick support cannot turn on unattended access, and `approval = never`
//! is only valid together with the unattended switch.

use serde::Deserialize;

use cbdesk_proto::AuthMethod;

use crate::error::SessionError;

/// Who has to approve a connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Always show a consent prompt.
    Always,
    /// Skip the prompt only for a known trusted key.
    UntrustedOnly,
    /// Never show a prompt. Requires [`SecurityConfig::unattended_enabled`].
    Never,
}

/// Host policy loaded from TOML.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    /// When the host shows a consent prompt.
    pub approval: ApprovalMode,
    /// Allow password, code, or trusted-device grants with no person present.
    #[serde(default)]
    pub unattended_enabled: bool,
    /// Require a second factor after the primary method succeeds.
    #[serde(default)]
    pub require_2fa: bool,
    /// One-off support session. Cannot be unattended.
    #[serde(default)]
    pub quick_support: bool,
}

impl SecurityConfig {
    /// Reject combinations the product is not allowed to run.
    pub fn validate(&self) -> Result<(), SessionError> {
        if self.quick_support && (self.unattended_enabled || self.approval == ApprovalMode::Never) {
            return Err(SessionError::QuickSupportForbidsUnattended);
        }
        if self.approval == ApprovalMode::Never && !self.unattended_enabled {
            return Err(SessionError::NeverRequiresUnattended);
        }
        Ok(())
    }
}

/// Parse TOML settings and run [`SecurityConfig::validate`].
pub fn parse_security_config(text: &str) -> Result<SecurityConfig, SessionError> {
    let config: SecurityConfig = toml::from_str(text).map_err(|_| SessionError::BadConfig)?;
    config.validate()?;
    Ok(config)
}

/// Stable method name stored in settings and logs.
pub fn auth_method_name(method: AuthMethod) -> Option<&'static str> {
    Some(match method {
        AuthMethod::Interactive => "approval",
        AuthMethod::PakeCode => "code",
        AuthMethod::OneTimePassword => "one_time_password",
        AuthMethod::TrustedKey => "trusted",
        AuthMethod::OpaquePassword => "permanent_password",
        AuthMethod::OrgCert => "org",
        AuthMethod::Totp => "totp",
        AuthMethod::Unspecified | AuthMethod::Unknown(_) => return None,
    })
}

/// Parse a method name written by [`auth_method_name`].
pub fn parse_auth_method(name: &str) -> Option<AuthMethod> {
    Some(match name {
        "approval" => AuthMethod::Interactive,
        "code" => AuthMethod::PakeCode,
        "one_time_password" => AuthMethod::OneTimePassword,
        "trusted" => AuthMethod::TrustedKey,
        "permanent_password" => AuthMethod::OpaquePassword,
        "org" => AuthMethod::OrgCert,
        "totp" => AuthMethod::Totp,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stealth_is_not_a_setting_and_unattended_rules_hold() {
        let stealth = "approval = \"always\"\nunattended_enabled = false\nstealth = true\n";
        assert_eq!(parse_security_config(stealth), Err(SessionError::BadConfig));
        let never = "approval = \"never\"\nunattended_enabled = false\n";
        assert_eq!(
            parse_security_config(never),
            Err(SessionError::NeverRequiresUnattended)
        );
        let quick = "approval = \"never\"\nunattended_enabled = true\nquick_support = true\n";
        assert_eq!(
            parse_security_config(quick),
            Err(SessionError::QuickSupportForbidsUnattended)
        );
        let ok =
            parse_security_config("approval = \"always\"\nquick_support = true\n").expect("ok");
        assert!(ok.quick_support);
        assert!(!ok.unattended_enabled);
    }

    #[test]
    fn method_names_round_trip() {
        let methods = [
            AuthMethod::Interactive,
            AuthMethod::PakeCode,
            AuthMethod::OneTimePassword,
            AuthMethod::TrustedKey,
            AuthMethod::OpaquePassword,
            AuthMethod::OrgCert,
            AuthMethod::Totp,
        ];
        for method in methods {
            let name = auth_method_name(method).expect("name");
            assert_eq!(parse_auth_method(name), Some(method));
        }
        assert_eq!(parse_auth_method("stealth"), None);
        assert_eq!(auth_method_name(AuthMethod::Unknown(9)), None);
    }
}

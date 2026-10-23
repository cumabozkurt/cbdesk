// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Consent-first host sessions.
//!
//! An invite ticket is not an input to this crate. Callers authenticate, then
//! either show a consent prompt or apply an explicit unattended policy. An
//! unanswered prompt closes after [`APPROVAL_TIMEOUT_MS`]. There is no setting
//! that suppresses the session indicator, and quick support cannot enable
//! unattended access.

mod config;
mod error;
mod host;
mod limit;
mod perms;

pub use config::{
    auth_method_name, parse_auth_method, parse_security_config, ApprovalMode, SecurityConfig,
};
pub use error::SessionError;
pub use host::{
    AuthStep, CloseReason, HostSession, PeerFacts, Phase, APPROVAL_TIMEOUT_MS, CONSENT_DELAY_MS,
};
pub use limit::{net_prefix, NetPrefix, RateLimiter};
pub use perms::PermissionSet;

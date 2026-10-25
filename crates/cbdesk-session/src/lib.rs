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

mod audit;
mod config;
mod controls;
mod deadline;
mod error;
mod host;
mod limit;
mod markup;
mod perms;
mod profile;
mod sides;

pub use audit::{verify_log, AccessLog, LogEntry};
pub use config::{
    auth_method_name, parse_auth_method, parse_security_config, ApprovalMode, SecurityConfig,
};
pub use controls::{
    open_terminal, open_tunnel, request_block_input, request_privacy_mode, request_recording,
    OsPermit, PermitKind,
};
pub use deadline::SessionDeadline;
pub use error::SessionError;
pub use host::{
    AuthStep, CloseReason, HostSession, PeerFacts, Phase, APPROVAL_TIMEOUT_MS, CONSENT_DELAY_MS,
};
pub use limit::{net_prefix, NetPrefix, RateLimiter};
pub use markup::{accept_stroke, Stroke};
pub use perms::PermissionSet;
pub use profile::{profile_permissions, AccessProfile};
pub use sides::{commit_side_switch, SideSwitch};

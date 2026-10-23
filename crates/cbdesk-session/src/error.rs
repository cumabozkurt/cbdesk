// SPDX-License-Identifier: AGPL-3.0-or-later

//! Session-layer errors. None of these hide a session.

use thiserror::Error;

/// The host refused a transition or a settings file.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The settings file was not the expected shape.
    #[error("security settings are invalid")]
    BadConfig,
    /// Quick support was combined with unattended access.
    #[error("quick support cannot enable unattended access")]
    QuickSupportForbidsUnattended,
    /// `approval = never` without the unattended switch.
    #[error("approval never requires unattended access to be enabled")]
    NeverRequiresUnattended,
    /// Authentication was started from the wrong phase.
    #[error("the session is not idle")]
    NotIdle,
    /// An authentication result arrived with no authentication in progress.
    #[error("the session is not authenticating")]
    NotAuthenticating,
    /// Approval was asked for when no prompt is showing.
    #[error("the session is not waiting for consent")]
    NotAwaitingConsent,
    /// The first-contact delay has not elapsed.
    #[error("consent is not available yet")]
    ConsentTooEarly,
    /// The approval window has not reached its deadline.
    #[error("the approval window is still open")]
    ApprovalStillOpen,
    /// The session was already closed.
    #[error("the session is already closed")]
    AlreadyClosed,
    /// The person approved an empty permission set.
    #[error("a grant needs at least one permission")]
    EmptyGrant,
    /// A first contact included a high-risk permission without the extra check.
    #[error("high-risk permissions need an explicit opt-in on first contact")]
    HighRiskNeedsOptIn,
    /// The permission cannot be granted by this version.
    #[error("permission is not grantable")]
    UnknownPermission,
    /// Host policy rejected the authentication method.
    #[error("host policy denied this authentication method")]
    PolicyDenied,
    /// The permission is not in the active grant.
    #[error("permission is not granted")]
    PermissionDenied,
    /// A channel was opened before the host granted the session.
    #[error("channel opened before the session was granted")]
    ChannelBeforeGrant,
    /// The operating system random source failed.
    #[error("operating system random source failed")]
    Rng,
    /// This peer or its network prefix is locked out.
    #[error("authentication is rate limited")]
    RateLimited,
    /// Too many handshakes are unfinished.
    #[error("too many handshakes are in progress")]
    TooManyHandshakes,
    /// A consent prompt was already shown during this minute.
    #[error("a consent prompt was already shown")]
    PromptTooSoon,
    /// Repeated rejections silenced prompts for this peer.
    #[error("consent prompts are silenced for this peer")]
    PromptSilenced,
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host-side session phases.
//!
//! A session indicator is visible while consent is pending and while the
//! session is active. [`HostSession::indicator_can_be_disabled`] is a
//! constant false. Media and other channels fail until a grant exists.

use cbdesk_proto::{AuthMethod, Permission};

use crate::config::{ApprovalMode, SecurityConfig};
use crate::error::SessionError;
use crate::perms::PermissionSet;

/// How long a stranger waits before Allow becomes available.
pub const CONSENT_DELAY_MS: u64 = 3_000;

/// How long the approval window stays open after it is shown.
///
/// The first-contact delay is inside this window. An unanswered prompt closes
/// at this deadline with [`CloseReason::TimedOut`]. The clock starts when the
/// prompt appears, not when Allow becomes clickable.
pub const APPROVAL_TIMEOUT_MS: u64 = 30_000;

/// Second-factor progress inside [`Phase::Authenticating`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthStep {
    /// The primary method has not succeeded yet.
    Primary,
    /// The primary method succeeded and TOTP is still required.
    Totp,
}

/// Why the host closed the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// The person at the host denied the prompt.
    Rejected,
    /// Policy rejected the method.
    Policy,
    /// The local side closed it.
    LocalClose,
    /// The person did not answer before [`APPROVAL_TIMEOUT_MS`].
    TimedOut,
}

/// What the host is doing right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No authentication has started.
    Idle,
    /// A method is in progress.
    Authenticating {
        /// Method offered by the controller.
        method: AuthMethod,
        /// Primary method, or TOTP when a second factor is required.
        step: AuthStep,
    },
    /// A person has to answer.
    AwaitingConsent {
        /// `approve` is refused before this timestamp.
        ready_at_ms: u64,
        /// True when this peer is not in the trust store.
        first_contact: bool,
        /// Method that reached the prompt.
        method: AuthMethod,
        /// `expire_unanswered` closes the session at this timestamp.
        deadline_ms: u64,
    },
    /// Permissions are in force.
    Active {
        /// Granted permissions.
        perms: PermissionSet,
        /// Random id for this grant. Tests should not pin the bytes.
        session_id: [u8; 16],
        /// True after [`HostSession::open_media`].
        media_started: bool,
    },
    /// Terminal phase.
    Closed {
        /// Why it closed.
        reason: CloseReason,
    },
}

/// Facts the policy needs besides the authentication method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerFacts {
    /// The peer's key is already in the host trust store.
    pub peer_known: bool,
}

/// One host-side session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSession {
    config: SecurityConfig,
    phase: Phase,
}

impl HostSession {
    /// Start idle after rejecting illegal settings.
    pub fn new(config: SecurityConfig) -> Result<Self, SessionError> {
        config.validate()?;
        Ok(Self {
            config,
            phase: Phase::Idle,
        })
    }

    /// Current phase.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// True while a person should see that a session exists or is being asked for.
    pub fn indicator_visible(&self) -> bool {
        matches!(
            self.phase,
            Phase::AwaitingConsent { .. } | Phase::Active { .. }
        )
    }

    /// The indicator has no off switch.
    pub const fn indicator_can_be_disabled() -> bool {
        false
    }

    /// Enter [`Phase::Authenticating`] from [`Phase::Idle`].
    pub fn start_authentication(&mut self, method: AuthMethod) -> Result<(), SessionError> {
        if !matches!(self.phase, Phase::Idle) {
            return Err(SessionError::NotIdle);
        }
        self.phase = Phase::Authenticating {
            method,
            step: AuthStep::Primary,
        };
        Ok(())
    }

    /// Record that the current authentication step succeeded.
    ///
    /// When `require_2fa` is set, the first success stays on the TOTP step.
    pub fn authentication_succeeded(
        &mut self,
        facts: &PeerFacts,
        now_ms: u64,
    ) -> Result<(), SessionError> {
        let (method, step) = match self.phase {
            Phase::Authenticating { method, step } => (method, step),
            _ => return Err(SessionError::NotAuthenticating),
        };
        if self.config.require_2fa && step == AuthStep::Primary {
            self.phase = Phase::Authenticating {
                method,
                step: AuthStep::Totp,
            };
            return Ok(());
        }
        self.finish_authentication(facts, now_ms, method)
    }

    /// Accept the prompt. Too-early and empty grants leave the phase unchanged.
    pub fn approve(
        &mut self,
        now_ms: u64,
        perms: &[Permission],
        explicit_high_risk: bool,
    ) -> Result<(), SessionError> {
        let (ready_at_ms, first_contact) = match self.phase {
            Phase::AwaitingConsent {
                ready_at_ms,
                first_contact,
                ..
            } => (ready_at_ms, first_contact),
            _ => return Err(SessionError::NotAwaitingConsent),
        };
        if now_ms < ready_at_ms {
            return Err(SessionError::ConsentTooEarly);
        }
        let granted = interactive_grant(perms, first_contact, explicit_high_risk)?;
        self.enter_active(granted)
    }

    /// Deny the prompt.
    pub fn reject(&mut self) -> Result<(), SessionError> {
        if !matches!(self.phase, Phase::AwaitingConsent { .. }) {
            return Err(SessionError::NotAwaitingConsent);
        }
        self.phase = Phase::Closed {
            reason: CloseReason::Rejected,
        };
        Ok(())
    }

    /// Close from any live phase.
    pub fn close_with(&mut self, reason: CloseReason) -> Result<(), SessionError> {
        if matches!(self.phase, Phase::Closed { .. }) {
            return Err(SessionError::AlreadyClosed);
        }
        self.phase = Phase::Closed { reason };
        Ok(())
    }

    /// When an unanswered prompt closes, if one is showing.
    pub fn approval_deadline_ms(&self) -> Option<u64> {
        match self.phase {
            Phase::AwaitingConsent { deadline_ms, .. } => Some(deadline_ms),
            _ => None,
        }
    }

    /// Close an unanswered prompt once `now_ms` reaches the deadline.
    ///
    /// The phase stays [`Phase::AwaitingConsent`] until that timestamp. Media
    /// stays closed.
    pub fn expire_unanswered(&mut self, now_ms: u64) -> Result<(), SessionError> {
        let deadline_ms = self
            .approval_deadline_ms()
            .ok_or(SessionError::NotAwaitingConsent)?;
        if now_ms < deadline_ms {
            return Err(SessionError::ApprovalStillOpen);
        }
        self.close_with(CloseReason::TimedOut)
    }

    /// `Granted` for the active phase. Absent until a person or an unattended
    /// policy has approved the session.
    pub fn granted_message(&self) -> Result<cbdesk_proto::Granted, SessionError> {
        match &self.phase {
            Phase::Active {
                perms, session_id, ..
            } => Ok(cbdesk_proto::Granted {
                perms: perms.granted(),
                session_id: session_id.to_vec(),
                ..cbdesk_proto::Granted::default()
            }),
            _ => Err(SessionError::ChannelBeforeGrant),
        }
    }

    /// Whether `perm` may be used. Fails before a grant exists.
    pub fn authorize(&self, perm: Permission) -> Result<(), SessionError> {
        match &self.phase {
            Phase::Active { perms, .. } if perms.contains(perm) => Ok(()),
            Phase::Active { .. } => Err(SessionError::PermissionDenied),
            _ => Err(SessionError::ChannelBeforeGrant),
        }
    }

    /// Open the media channel. Fails unless the session is active.
    pub fn open_media(&mut self) -> Result<(), SessionError> {
        match &mut self.phase {
            Phase::Active { media_started, .. } => {
                *media_started = true;
                Ok(())
            }
            _ => Err(SessionError::ChannelBeforeGrant),
        }
    }

    fn finish_authentication(
        &mut self,
        facts: &PeerFacts,
        now_ms: u64,
        method: AuthMethod,
    ) -> Result<(), SessionError> {
        match self.config.approval {
            ApprovalMode::Always => self.enter_consent(facts, now_ms, method),
            ApprovalMode::UntrustedOnly => self.finish_untrusted_only(facts, now_ms, method),
            ApprovalMode::Never => self.finish_never(facts, method),
        }
    }

    fn finish_untrusted_only(
        &mut self,
        facts: &PeerFacts,
        now_ms: u64,
        method: AuthMethod,
    ) -> Result<(), SessionError> {
        if facts.peer_known && method == AuthMethod::TrustedKey {
            return self.enter_active(PermissionSet::safe_unattended());
        }
        self.enter_consent(facts, now_ms, method)
    }

    fn finish_never(&mut self, facts: &PeerFacts, method: AuthMethod) -> Result<(), SessionError> {
        let allowed = match method {
            AuthMethod::OpaquePassword
            | AuthMethod::OneTimePassword
            | AuthMethod::OrgCert
            | AuthMethod::PakeCode => true,
            AuthMethod::TrustedKey => facts.peer_known,
            _ => false,
        };
        if !allowed {
            self.phase = Phase::Closed {
                reason: CloseReason::Policy,
            };
            return Err(SessionError::PolicyDenied);
        }
        self.enter_active(PermissionSet::safe_unattended())
    }

    fn enter_consent(
        &mut self,
        facts: &PeerFacts,
        now_ms: u64,
        method: AuthMethod,
    ) -> Result<(), SessionError> {
        let delay = if facts.peer_known {
            0
        } else {
            CONSENT_DELAY_MS
        };
        self.phase = Phase::AwaitingConsent {
            ready_at_ms: now_ms.saturating_add(delay),
            first_contact: !facts.peer_known,
            method,
            deadline_ms: now_ms.saturating_add(APPROVAL_TIMEOUT_MS),
        };
        Ok(())
    }

    fn enter_active(&mut self, perms: PermissionSet) -> Result<(), SessionError> {
        self.phase = Phase::Active {
            perms,
            session_id: random_session_id()?,
            media_started: false,
        };
        Ok(())
    }
}

fn interactive_grant(
    perms: &[Permission],
    first_contact: bool,
    explicit_high_risk: bool,
) -> Result<PermissionSet, SessionError> {
    if perms.is_empty() {
        return Err(SessionError::EmptyGrant);
    }
    let mut set = PermissionSet::empty();
    for perm in perms {
        if perm.bit().is_none() {
            return Err(SessionError::UnknownPermission);
        }
        if perm.is_high_risk() && first_contact && !explicit_high_risk {
            return Err(SessionError::HighRiskNeedsOptIn);
        }
        set.insert(*perm);
    }
    Ok(set)
}

fn random_session_id() -> Result<[u8; 16], SessionError> {
    let mut id = [0_u8; 16];
    getrandom::getrandom(&mut id).map_err(|_| SessionError::Rng)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(approval: ApprovalMode, unattended: bool, require_2fa: bool) -> SecurityConfig {
        SecurityConfig {
            approval,
            unattended_enabled: unattended,
            require_2fa,
            quick_support: false,
        }
    }

    fn session(approval: ApprovalMode, unattended: bool) -> HostSession {
        HostSession::new(config(approval, unattended, false)).expect("session")
    }

    #[test]
    fn first_contact_consent_gates_channels_and_high_risk() {
        let mut host = session(ApprovalMode::Always, false);
        assert!(host.open_media().is_err());
        assert!(!host.indicator_visible());
        host.start_authentication(AuthMethod::Interactive)
            .expect("start");
        host.authentication_succeeded(&PeerFacts { peer_known: false }, 1_000)
            .expect("auth");
        assert!(host.indicator_visible());
        assert!(matches!(
            host.phase(),
            Phase::AwaitingConsent {
                ready_at_ms: 4_000,
                first_contact: true,
                ..
            }
        ));
        assert_eq!(
            host.approve(3_999, &[Permission::View], false),
            Err(SessionError::ConsentTooEarly)
        );
        assert!(matches!(host.phase(), Phase::AwaitingConsent { .. }));
        assert_eq!(
            host.approve(4_000, &[], false),
            Err(SessionError::EmptyGrant)
        );
        assert_eq!(
            host.approve(4_000, &[Permission::FileWrite], false),
            Err(SessionError::HighRiskNeedsOptIn)
        );
        host.approve(4_000, &[Permission::View, Permission::FileWrite], true)
            .expect("approve");
        assert!(host.indicator_visible());
        assert!(!HostSession::indicator_can_be_disabled());
        host.authorize(Permission::View).expect("view");
        host.authorize(Permission::FileWrite).expect("write");
        host.open_media().expect("media");
        assert!(matches!(
            host.phase(),
            Phase::Active {
                media_started: true,
                ..
            }
        ));
    }

    #[test]
    fn known_peer_has_no_delay_and_unattended_strips_high_risk() {
        let mut known = session(ApprovalMode::Always, false);
        known
            .start_authentication(AuthMethod::Interactive)
            .expect("start");
        known
            .authentication_succeeded(&PeerFacts { peer_known: true }, 50)
            .expect("auth");
        known
            .approve(50, &[Permission::View], false)
            .expect("approve");

        let mut trusted = session(ApprovalMode::UntrustedOnly, false);
        trusted
            .start_authentication(AuthMethod::TrustedKey)
            .expect("start");
        trusted
            .authentication_succeeded(&PeerFacts { peer_known: true }, 0)
            .expect("auth");
        trusted.authorize(Permission::View).expect("view");
        trusted.authorize(Permission::Input).expect("input");
        trusted.authorize(Permission::Clipboard).expect("clip");
        trusted.authorize(Permission::Audio).expect("audio");
        trusted.authorize(Permission::Sysinfo).expect("info");
        assert_eq!(
            trusted.authorize(Permission::Terminal),
            Err(SessionError::PermissionDenied)
        );
        assert!(trusted.indicator_visible());

        let mut stranger = HostSession::new(config(ApprovalMode::Never, true, false)).expect("cfg");
        stranger
            .start_authentication(AuthMethod::PakeCode)
            .expect("start");
        stranger
            .authentication_succeeded(&PeerFacts { peer_known: false }, 0)
            .expect("auth");
        assert_eq!(
            stranger.authorize(Permission::FileWrite),
            Err(SessionError::PermissionDenied)
        );
        assert_eq!(
            stranger.start_authentication(AuthMethod::TrustedKey),
            Err(SessionError::NotIdle)
        );
    }

    #[test]
    fn never_rejects_strangers_using_trusted_keys_and_interactive() {
        let mut host = HostSession::new(config(ApprovalMode::Never, true, true)).expect("cfg");
        host.start_authentication(AuthMethod::TrustedKey)
            .expect("start");
        host.authentication_succeeded(&PeerFacts { peer_known: false }, 0)
            .expect("primary");
        assert!(matches!(
            host.phase(),
            Phase::Authenticating {
                step: AuthStep::Totp,
                ..
            }
        ));
        assert_eq!(
            host.authentication_succeeded(&PeerFacts { peer_known: false }, 0),
            Err(SessionError::PolicyDenied)
        );
        assert!(matches!(
            host.phase(),
            Phase::Closed {
                reason: CloseReason::Policy
            }
        ));
    }

    #[test]
    fn unanswered_approval_closes_at_thirty_seconds() {
        let mut host = session(ApprovalMode::Always, false);
        assert_eq!(
            host.expire_unanswered(0),
            Err(SessionError::NotAwaitingConsent)
        );
        host.start_authentication(AuthMethod::Interactive)
            .expect("start");
        host.authentication_succeeded(&PeerFacts { peer_known: false }, 1_000)
            .expect("auth");
        assert_eq!(
            host.approval_deadline_ms(),
            Some(1_000 + APPROVAL_TIMEOUT_MS)
        );
        assert_eq!(APPROVAL_TIMEOUT_MS, 30_000);
        assert_eq!(
            host.expire_unanswered(1_000 + APPROVAL_TIMEOUT_MS - 1),
            Err(SessionError::ApprovalStillOpen)
        );
        assert!(host.indicator_visible());
        assert!(host.granted_message().is_err());
        host.expire_unanswered(1_000 + APPROVAL_TIMEOUT_MS)
            .expect("timeout");
        assert!(matches!(
            host.phase(),
            Phase::Closed {
                reason: CloseReason::TimedOut
            }
        ));
        assert!(!host.indicator_visible());
        assert!(host.authorize(Permission::View).is_err());
        assert!(host.open_media().is_err());

        let mut known = session(ApprovalMode::Always, false);
        known
            .start_authentication(AuthMethod::Interactive)
            .expect("start");
        known
            .authentication_succeeded(&PeerFacts { peer_known: true }, 50)
            .expect("auth");
        assert_eq!(known.approval_deadline_ms(), Some(50 + APPROVAL_TIMEOUT_MS));
    }

    #[test]
    fn reject_hides_the_indicator() {
        let mut host = session(ApprovalMode::Always, false);
        host.start_authentication(AuthMethod::Interactive)
            .expect("start");
        host.authentication_succeeded(&PeerFacts { peer_known: true }, 0)
            .expect("auth");
        host.reject().expect("reject");
        assert!(!host.indicator_visible());
        assert!(host.open_media().is_err());
    }
}

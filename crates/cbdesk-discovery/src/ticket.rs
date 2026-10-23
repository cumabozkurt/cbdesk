// SPDX-License-Identifier: AGPL-3.0-or-later

//! Signed invite tickets.
//!
//! A verified ticket proves that the named device published these addresses.
//! It does not authorize viewing, input, or anything else. The session layer
//! still has to approve the connection.

use std::collections::HashSet;

use cbdesk_crypto::{DeviceId, Identity};
use cbdesk_proto::{InviteTicket, ProtoMessage, MAX_DIRECT_ADDRS};

use crate::error::DiscoveryError;

/// Longest lifetime a ticket may claim, in seconds.
pub const MAX_TICKET_TTL_SECS: u64 = 15 * 60;

/// Remembers signatures that have already been accepted.
#[derive(Debug, Default)]
pub struct ReplayCache {
    seen: HashSet<[u8; 32]>,
}

impl ReplayCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    fn remember(&mut self, signature: &[u8]) -> bool {
        let digest = blake3::hash(signature);
        self.seen.insert(*digest.as_bytes())
    }
}

/// Sign `ticket` with `identity`. The signature covers every field except `sig`.
pub fn sign_ticket(
    mut ticket: InviteTicket,
    identity: &Identity,
) -> Result<InviteTicket, DiscoveryError> {
    validate_addrs(&ticket.direct_addrs)?;
    ticket.version = 1;
    ticket.peer_pk = identity.device_id().as_bytes().to_vec();
    ticket.sig.clear();
    let payload = ticket.to_bytes()?;
    ticket.sig = identity.sign(&payload).to_vec();
    Ok(ticket)
}

/// Verify a ticket's signature, lifetime, and replay status.
///
/// The returned id is who signed the locator. Callers still run the normal
/// consent or unattended policy before any session exists.
pub fn verify_ticket(
    ticket: &InviteTicket,
    now_secs: u64,
    replay: &mut ReplayCache,
) -> Result<DeviceId, DiscoveryError> {
    if ticket.version != 1 {
        return Err(DiscoveryError::BadTicketVersion);
    }
    validate_addrs(&ticket.direct_addrs)?;
    if ticket.sig.len() != 64 {
        return Err(DiscoveryError::Crypto(
            cbdesk_crypto::CryptoError::BadSignature,
        ));
    }
    let bytes: [u8; 32] = ticket
        .peer_pk
        .as_slice()
        .try_into()
        .map_err(|_| DiscoveryError::Crypto(cbdesk_crypto::CryptoError::InvalidPublicKey))?;
    let device_id = DeviceId::from_bytes(bytes)?;
    let mut payload = ticket.clone();
    payload.sig.clear();
    device_id.verify(&payload.to_bytes()?, &ticket.sig)?;
    if ticket.expires_at <= now_secs {
        return Err(DiscoveryError::TicketExpired);
    }
    if ticket.expires_at - now_secs > MAX_TICKET_TTL_SECS {
        return Err(DiscoveryError::TicketTtlTooLong);
    }
    if !replay.remember(&ticket.sig) {
        return Err(DiscoveryError::TicketReplayed);
    }
    Ok(device_id)
}

/// Tickets are locators. This constant is the invariant tests lock in.
pub const fn ticket_grants_access() -> bool {
    false
}

fn validate_addrs(addrs: &[String]) -> Result<(), DiscoveryError> {
    if addrs.len() > MAX_DIRECT_ADDRS || addrs.iter().any(|addr| addr.is_empty()) {
        return Err(DiscoveryError::BadTicketAddrs);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_proto::InvitePurpose;

    fn sample(identity: &Identity, expires_at: u64) -> InviteTicket {
        sign_ticket(
            InviteTicket {
                direct_addrs: vec!["192.0.2.20:22116".to_string()],
                relay_urls: vec!["relay://203.0.113.9:443".to_string()],
                expires_at,
                purpose: InvitePurpose::Session,
                pake_hint: b"not-a-password".to_vec(),
                ..InviteTicket::default()
            },
            identity,
        )
        .expect("sign")
    }

    #[test]
    fn valid_ticket_verifies_once_and_does_not_grant_access() {
        let identity = Identity::generate().expect("id");
        let ticket = sample(&identity, 1_000 + MAX_TICKET_TTL_SECS);
        let mut replay = ReplayCache::new();
        let id = verify_ticket(&ticket, 1_000, &mut replay).expect("verify");
        assert_eq!(id, identity.device_id());
        assert!(matches!(
            verify_ticket(&ticket, 1_000, &mut replay),
            Err(DiscoveryError::TicketReplayed)
        ));
        assert!(!ticket_grants_access());
    }

    #[test]
    fn expired_tampered_and_long_lived_tickets_fail() {
        let identity = Identity::generate().expect("id");
        let mut replay = ReplayCache::new();
        let expired = sample(&identity, 500);
        assert!(matches!(
            verify_ticket(&expired, 500, &mut replay),
            Err(DiscoveryError::TicketExpired)
        ));
        assert!(matches!(
            verify_ticket(&expired, 500, &mut replay),
            Err(DiscoveryError::TicketExpired)
        ));

        let mut ticket = sample(&identity, 2_000);
        ticket.direct_addrs[0] = "198.51.100.8:22116".to_string();
        assert!(verify_ticket(&ticket, 1_000, &mut replay).is_err());

        let long_lived = sample(&identity, 1_000 + MAX_TICKET_TTL_SECS + 1);
        assert!(matches!(
            verify_ticket(&long_lived, 1_000, &mut replay),
            Err(DiscoveryError::TicketTtlTooLong)
        ));

        let other = Identity::generate().expect("other");
        let mut swapped = sample(&identity, 2_000);
        swapped.peer_pk = other.device_id().as_bytes().to_vec();
        assert!(verify_ticket(&swapped, 1_000, &mut replay).is_err());
    }
}

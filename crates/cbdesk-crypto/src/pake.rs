// SPDX-License-Identifier: AGPL-3.0-or-later

//! SPAKE2 pairing for a short code.
//!
//! The password is the two lowercase words. The public slot is not part of
//! the password. It is bound into the transcript with both Ed25519 keys.
//! `finish` of SPAKE2 succeeds even when the passwords differ, so each side
//! compares a confirmation tag and then wipes the shared key. This module
//! calls the `spake2` crate. It does not define its own group arithmetic.

use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroize;

use crate::CryptoError;

const HOST_LABEL: &[u8] = b"cbdesk-host";
const CONTROLLER_LABEL: &[u8] = b"cbdesk-controller";
const CONFIRM_LABEL: &[u8] = b"cbdesk-spake2-confirm-v1";

/// Which peer is speaking. The host is SPAKE2 side A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The computer that showed the short code.
    Host,
    /// The computer that typed the short code.
    Controller,
}

/// Tags that prove both sides derived the same key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confirmation {
    /// Tag this side sends.
    pub local_tag: [u8; 32],
    /// Tag this side expects from the peer.
    pub peer_tag: [u8; 32],
}

/// One in-progress SPAKE2 exchange.
pub struct Handshake {
    state: Spake2<Ed25519Group>,
    transcript: Vec<u8>,
    side: Side,
}

impl Handshake {
    /// Begin as the host (side A). `outbound` is the message to send.
    pub fn start_host(password: &[u8], transcript: &[u8]) -> Result<(Self, Vec<u8>), CryptoError> {
        Self::start(Side::Host, password, transcript)
    }

    /// Begin as the controller (side B). `outbound` is the message to send.
    pub fn start_controller(
        password: &[u8],
        transcript: &[u8],
    ) -> Result<(Self, Vec<u8>), CryptoError> {
        Self::start(Side::Controller, password, transcript)
    }

    /// Consume the peer message and build confirmation tags.
    ///
    /// The SPAKE2 key is wiped before this returns. A wrong password still
    /// produces tags; they will not match the peer's tags.
    pub fn finish(self, peer_message: &[u8]) -> Result<Confirmation, CryptoError> {
        let mut key = self
            .state
            .finish(peer_message)
            .map_err(|_| CryptoError::Pairing)?;
        let tags = confirmation_tags(&key, &self.transcript, self.side);
        key.zeroize();
        Ok(tags)
    }

    fn start(
        side: Side,
        password: &[u8],
        transcript: &[u8],
    ) -> Result<(Self, Vec<u8>), CryptoError> {
        if password.is_empty() || transcript.is_empty() {
            return Err(CryptoError::Pairing);
        }
        let (id_a, id_b) = identities(transcript);
        let password = Password::new(password);
        let (state, outbound) = match side {
            Side::Host => Spake2::<Ed25519Group>::start_a(&password, &id_a, &id_b),
            Side::Controller => Spake2::<Ed25519Group>::start_b(&password, &id_a, &id_b),
        };
        if outbound.is_empty() || outbound.len() > 256 {
            return Err(CryptoError::Pairing);
        }
        Ok((
            Self {
                state,
                transcript: transcript.to_vec(),
                side,
            },
            outbound,
        ))
    }
}

/// `word-word` in the stored lowercase form. The slot is not included.
pub fn password_bytes(words: &[String; 2]) -> Vec<u8> {
    format!("{}-{}", words[0], words[1]).into_bytes()
}

/// Slot, host key, then controller key. The display string is not included.
pub fn transcript(slot: u16, host_pk: &[u8; 32], controller_pk: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + 64);
    out.extend_from_slice(&slot.to_be_bytes());
    out.extend_from_slice(host_pk);
    out.extend_from_slice(controller_pk);
    out
}

/// Constant-time compare for two confirmation tags.
pub fn tags_match(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (one, other) in left.iter().zip(right.iter()) {
        diff |= one ^ other;
    }
    diff == 0
}

fn identities(transcript: &[u8]) -> (Identity, Identity) {
    let mut host = HOST_LABEL.to_vec();
    host.extend_from_slice(transcript);
    let mut controller = CONTROLLER_LABEL.to_vec();
    controller.extend_from_slice(transcript);
    (Identity::new(&host), Identity::new(&controller))
}

fn confirmation_tags(key: &[u8], transcript: &[u8], side: Side) -> Confirmation {
    let mut mac_key = *blake3::hash(key).as_bytes();
    let host = role_tag(&mac_key, transcript, b"host");
    let controller = role_tag(&mac_key, transcript, b"controller");
    mac_key.zeroize();
    match side {
        Side::Host => Confirmation {
            local_tag: host,
            peer_tag: controller,
        },
        Side::Controller => Confirmation {
            local_tag: controller,
            peer_tag: host,
        },
    }
}

fn role_tag(mac_key: &[u8; 32], transcript: &[u8], role: &[u8]) -> [u8; 32] {
    let mut message = CONFIRM_LABEL.to_vec();
    message.extend_from_slice(transcript);
    message.extend_from_slice(role);
    *blake3::keyed_hash(mac_key, &message).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Vec<u8>, Vec<u8>) {
        let host = [1_u8; 32];
        let controller = [2_u8; 32];
        let transcript = transcript(428, &host, &controller);
        let password = password_bytes(&["elma".into(), "armut".into()]);
        (password, transcript)
    }

    #[test]
    fn matching_words_confirm_and_a_wrong_word_does_not() {
        let (password, transcript) = sample();
        let (host, host_msg) = Handshake::start_host(&password, &transcript).expect("host");
        let (controller, controller_msg) =
            Handshake::start_controller(&password, &transcript).expect("controller");
        let host_tags = host.finish(&controller_msg).expect("host finish");
        let controller_tags = controller.finish(&host_msg).expect("controller finish");
        assert!(tags_match(&host_tags.local_tag, &controller_tags.peer_tag));
        assert!(tags_match(&controller_tags.local_tag, &host_tags.peer_tag));

        let wrong = password_bytes(&["elma".into(), "deniz".into()]);
        let (host, host_msg) = Handshake::start_host(&password, &transcript).expect("host");
        let (controller, controller_msg) =
            Handshake::start_controller(&wrong, &transcript).expect("controller");
        let host_tags = host.finish(&controller_msg).expect("host finish");
        let controller_tags = controller.finish(&host_msg).expect("controller finish");
        assert!(!tags_match(&host_tags.peer_tag, &controller_tags.local_tag));
    }

    #[test]
    fn the_slot_and_the_keys_are_in_the_transcript() {
        let (password, bound) = sample();
        let (host, host_msg) = Handshake::start_host(&password, &bound).expect("host");
        let other = transcript(429, &[1_u8; 32], &[2_u8; 32]);
        let (controller, controller_msg) =
            Handshake::start_controller(&password, &other).expect("controller");
        let host_tags = host.finish(&controller_msg).expect("finish");
        let controller_tags = controller.finish(&host_msg).expect("finish");
        assert!(!tags_match(&host_tags.peer_tag, &controller_tags.local_tag));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Errors from discovery objects. None of these grant a session.

use thiserror::Error;

use cbdesk_crypto::CryptoError;
use cbdesk_proto::ProtoError;

/// A ticket, code, announcement, or DHT record was rejected.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// The protobuf codec rejected the bytes.
    #[error(transparent)]
    Proto(#[from] ProtoError),
    /// A signature or public key was not valid.
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    /// The operating system random source failed.
    #[error("operating system random source failed")]
    Rng,
    /// The word list shipped with this build failed its own checks.
    #[error("word list is invalid: {0}")]
    Wordlist(&'static str),
    /// A short code was not `NNN-WORD-WORD` for the selected language.
    #[error("short code is invalid")]
    BadCode,
    /// A slot number was outside `0..=999`.
    #[error("slot number is outside 0..=999")]
    SlotOutOfRange,
    /// The ticket version is not 1.
    #[error("invite ticket version is not supported")]
    BadTicketVersion,
    /// The ticket has too many addresses, or an address is empty.
    #[error("invite ticket addresses are invalid")]
    BadTicketAddrs,
    /// The signature was valid but the ticket is no longer a locator.
    #[error("invite ticket has expired")]
    TicketExpired,
    /// The ticket claims a lifetime longer than the allowed maximum.
    #[error("invite ticket lifetime is too long")]
    TicketTtlTooLong,
    /// This signature was already accepted.
    #[error("invite ticket was already used")]
    TicketReplayed,
    /// The LAN announcement bytes were not a CBDesk packet.
    #[error("LAN announcement is invalid")]
    BadLanPacket,
    /// An mDNS message was not a CBDesk query or announcement.
    #[error("mDNS message is invalid")]
    BadMdns,
    /// A UDP socket could not be opened or used.
    #[error("LAN socket failed: {0}")]
    LanIo(String),
    /// DHT publishing under a blinded key is not available yet.
    #[error("DHT public-key blinding is not enabled until ADR-007 is accepted")]
    BlindingPendingReview,
    /// Isolated mode does not publish a DHT record.
    #[error("isolated mode does not publish a DHT record")]
    IsolatedNetwork,
    /// The record contains direct addresses while IP publishing is off.
    #[error("DHT record withholds IP but still contains a direct address")]
    IpNotPublished,
    /// The signed DHT bytes were not a valid address record.
    #[error("DHT record is invalid")]
    BadDhtRecord,
}

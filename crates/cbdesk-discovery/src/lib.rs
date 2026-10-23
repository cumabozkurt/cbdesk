// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Finding a peer is separate from being allowed to control it.
//!
//! Invite tickets, LAN announcements, short-code slots, and DHT records all
//! answer "where is this device?". The session layer answers "may this device
//! connect?". A verified ticket never skips consent.

mod dht;
mod error;
mod lan;
mod mdns;
mod presence;
mod slot;
mod ticket;
mod words;

pub use dht::{sign_dht_record, verify_dht_record, DhtKeyMode};
pub use error::DiscoveryError;
pub use lan::{
    lan_should_announce, LanPacket, MdnsTxt, NetworkProfile, GENERIC_DEVICE_NAME,
    LAN_BROADCAST_PORT, MDNS_IPV4, MDNS_PORT, MDNS_SERVICE_TYPE,
};
pub use mdns::{
    decode_mdns, encode_announcement, encode_browse_query, MdnsAnnouncement, MdnsMessage,
};
pub use presence::{
    DiscoveredPeer, DiscoveryPath, Incoming, LanPresence, LanPresenceConfig, LanSocketPlan,
};
pub use slot::{slot_epoch, slot_key, SLOT_EPOCH_SECS};
pub use ticket::{
    sign_ticket, ticket_grants_access, verify_ticket, ReplayCache, MAX_TICKET_TTL_SECS,
};
pub use words::{parse_code, word_list, Language, ShortCode};

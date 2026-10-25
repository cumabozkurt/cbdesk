// SPDX-License-Identifier: AGPL-3.0-or-later

//! Handshake and media-header messages for protocol `cbdesk/1`.
//!
//! Field numbers match `proto/cbdesk/v1/session.proto`. Zero-valued singular
//! fields are omitted on encode, which is the proto3 rule. Unknown field
//! numbers are kept on the message and written back after the known fields.

use crate::enums::{AuthMethod, Capability, Codec, ErrorCode, InvitePurpose, Permission, Platform};
use crate::error::ProtoError;
use crate::limits::CONTROL_MAX_BYTES;
use crate::wire::{self, DecodedMessage, Fields, UnknownFields};

/// A protobuf message with a hard size cap.
pub trait ProtoMessage: Sized + PartialEq {
    /// Largest accepted encoding of this message.
    const MAX_BYTES: usize;

    /// Append the protobuf encoding to `out`.
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError>;

    /// Decode from already-parsed fields. Implementations must keep leftovers.
    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError>;

    /// Encode and reject a result past [`Self::MAX_BYTES`].
    fn to_bytes(&self) -> Result<Vec<u8>, ProtoError> {
        let mut out = Vec::new();
        self.encode(&mut out)?;
        if out.len() > Self::MAX_BYTES {
            return Err(ProtoError::TooLarge {
                len: out.len(),
                limit: Self::MAX_BYTES,
            });
        }
        Ok(out)
    }

    /// Decode a buffer, rejecting anything past [`Self::MAX_BYTES`] first.
    fn from_bytes(bytes: &[u8]) -> Result<Self, ProtoError> {
        if bytes.len() > Self::MAX_BYTES {
            return Err(ProtoError::TooLarge {
                len: bytes.len(),
                limit: Self::MAX_BYTES,
            });
        }
        let mut fields = Fields::parse(bytes)?;
        Self::decode_fields(&mut fields)
    }
}

fn pack_enums<T: Copy>(values: &[T], to_i32: impl Fn(T) -> i32) -> Vec<i32> {
    values.iter().copied().map(to_i32).collect()
}

fn unpack_enums<T>(values: Vec<i32>, from_i32: impl Fn(i32) -> T) -> Vec<T> {
    values.into_iter().map(from_i32).collect()
}

/// First message from the controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    /// Major protocol version. Incompatible when this differs by more than one.
    pub proto_major: u32,
    /// Minor protocol version. Additive changes only.
    pub proto_minor: u32,
    /// Application version string, for logs and the consent window.
    pub app_version: String,
    /// Controller platform.
    pub platform: Platform,
    /// What the controller can do.
    pub caps: Vec<Capability>,
    /// Permissions the controller is asking for. The host may grant fewer.
    pub wanted_perms: Vec<Permission>,
    /// Authentication methods the controller is prepared to run.
    pub auth_offer: Vec<AuthMethod>,
    /// Device name ciphertext. Empty until authentication has finished.
    pub device_name_enc: Vec<u8>,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for Hello {
    fn default() -> Self {
        Self {
            proto_major: 0,
            proto_minor: 0,
            app_version: String::new(),
            platform: Platform::Unspecified,
            caps: Vec::new(),
            wanted_perms: Vec::new(),
            auth_offer: Vec::new(),
            device_name_enc: Vec::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for Hello {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.proto_major);
        wire::write_u32(out, 2, self.proto_minor);
        wire::write_string(out, 3, &self.app_version);
        wire::write_i32(out, 4, self.platform.as_i32());
        wire::write_packed_i32(out, 5, &pack_enums(&self.caps, Capability::as_i32));
        wire::write_packed_i32(out, 6, &pack_enums(&self.wanted_perms, Permission::as_i32));
        wire::write_packed_i32(out, 7, &pack_enums(&self.auth_offer, AuthMethod::as_i32));
        if !self.device_name_enc.is_empty() {
            wire::write_bytes(out, 8, &self.device_name_enc);
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            proto_major: fields.take_u32(1)?,
            proto_minor: fields.take_u32(2)?,
            app_version: fields.take_string(3)?,
            platform: Platform::from_i32(fields.take_i32(4)?),
            caps: unpack_enums(fields.take_packed_i32(5)?, Capability::from_i32),
            wanted_perms: unpack_enums(fields.take_packed_i32(6)?, Permission::from_i32),
            auth_offer: unpack_enums(fields.take_packed_i32(7)?, AuthMethod::from_i32),
            device_name_enc: fields.take_bytes(8)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Host answer to [`Hello`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelloAck {
    /// Host major version.
    pub proto_major: u32,
    /// Host minor version.
    pub proto_minor: u32,
    /// What the host can do.
    pub caps: Vec<Capability>,
    /// Methods the controller must complete before a grant.
    pub auth_required: Vec<AuthMethod>,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for HelloAck {
    fn default() -> Self {
        Self {
            proto_major: 0,
            proto_minor: 0,
            caps: Vec::new(),
            auth_required: Vec::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for HelloAck {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.proto_major);
        wire::write_u32(out, 2, self.proto_minor);
        wire::write_packed_i32(out, 3, &pack_enums(&self.caps, Capability::as_i32));
        wire::write_packed_i32(out, 4, &pack_enums(&self.auth_required, AuthMethod::as_i32));
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            proto_major: fields.take_u32(1)?,
            proto_minor: fields.take_u32(2)?,
            caps: unpack_enums(fields.take_packed_i32(3)?, Capability::from_i32),
            auth_required: unpack_enums(fields.take_packed_i32(4)?, AuthMethod::from_i32),
            unknown: fields.finish_unknown(),
        })
    }
}

/// One display on the host.
#[derive(Clone, Debug)]
pub struct DisplayInfo {
    /// Stable id for this session.
    pub id: u32,
    /// Human-readable name. Empty if the host did not share one.
    pub name: String,
    /// Left edge in the host virtual desktop.
    pub x: i32,
    /// Top edge in the host virtual desktop.
    pub y: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Scale factor. Must be finite.
    pub scale: f32,
    /// The host's primary display.
    pub primary: bool,
    /// The display is HDR.
    pub hdr: bool,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl PartialEq for DisplayInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
            && self.scale.to_bits() == other.scale.to_bits()
            && self.primary == other.primary
            && self.hdr == other.hdr
            && self.unknown == other.unknown
    }
}

impl Default for DisplayInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            scale: 0.0,
            primary: false,
            hdr: false,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for DisplayInfo {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.id);
        wire::write_string(out, 2, &self.name);
        wire::write_i32(out, 3, self.x);
        wire::write_i32(out, 4, self.y);
        wire::write_u32(out, 5, self.width);
        wire::write_u32(out, 6, self.height);
        wire::write_f32(out, 7, self.scale)?;
        wire::write_bool(out, 8, self.primary);
        wire::write_bool(out, 9, self.hdr);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            id: fields.take_u32(1)?,
            name: fields.take_string(2)?,
            x: fields.take_i32(3)?,
            y: fields.take_i32(4)?,
            width: fields.take_u32(5)?,
            height: fields.take_u32(6)?,
            scale: fields.take_f32(7)?,
            primary: fields.take_bool(8)?,
            hdr: fields.take_bool(9)?,
            unknown: fields.finish_unknown(),
        })
    }
}

impl DecodedMessage for DisplayInfo {
    fn decode_limited(bytes: &[u8]) -> Result<Self, ProtoError> {
        Self::from_bytes(bytes)
    }
}

/// The host's decision to open a session.
///
/// Sending this message means a person approved the session, or an explicit
/// unattended policy already did. It is not produced from an invite ticket.
#[derive(Clone, Debug, PartialEq)]
pub struct Granted {
    /// Permissions now in force.
    pub perms: Vec<Permission>,
    /// Unix seconds when the grant ends. Zero means no protocol-level expiry.
    pub expires_at: u64,
    /// Random session identifier.
    pub session_id: Vec<u8>,
    /// Displays the controller may ask to see.
    pub displays: Vec<DisplayInfo>,
    /// Short-lived token used only to resume this same session.
    pub reconnect_token: Vec<u8>,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for Granted {
    fn default() -> Self {
        Self {
            perms: Vec::new(),
            expires_at: 0,
            session_id: Vec::new(),
            displays: Vec::new(),
            reconnect_token: Vec::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for Granted {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_packed_i32(out, 1, &pack_enums(&self.perms, Permission::as_i32));
        wire::write_u64(out, 2, self.expires_at);
        if !self.session_id.is_empty() {
            wire::write_bytes(out, 3, &self.session_id);
        }
        for display in &self.displays {
            let bytes = display.to_bytes()?;
            wire::write_message(out, 4, &bytes);
        }
        if !self.reconnect_token.is_empty() {
            wire::write_bytes(out, 5, &self.reconnect_token);
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            perms: unpack_enums(fields.take_packed_i32(1)?, Permission::from_i32),
            expires_at: fields.take_u64(2)?,
            session_id: fields.take_bytes(3)?,
            displays: fields.take_messages(4)?,
            reconnect_token: fields.take_bytes(5)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// The host changed permissions while the session was open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionChanged {
    /// The full new permission set, not a delta.
    pub perms: Vec<Permission>,
    /// Short reason shown to the controller. Not a secret.
    pub reason: String,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for PermissionChanged {
    fn default() -> Self {
        Self {
            perms: Vec::new(),
            reason: String::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for PermissionChanged {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_packed_i32(out, 1, &pack_enums(&self.perms, Permission::as_i32));
        wire::write_string(out, 2, &self.reason);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            perms: unpack_enums(fields.take_packed_i32(1)?, Permission::from_i32),
            reason: fields.take_string(2)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Header placed in front of one encoded video frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoFrameHeader {
    /// Display this frame belongs to.
    pub display_id: u32,
    /// Codec of the payload that follows.
    pub codec: Codec,
    /// Monotonic frame number for this display.
    pub frame_no: u64,
    /// Host capture time, microseconds.
    pub capture_ts_us: u64,
    /// True when the payload is a full-frame keyframe.
    ///
    /// A region update clears this flag and sets `damage_width`. The region
    /// bytes are themselves a keyframe of the crop. A viewer that ignores
    /// `damage_width` must not be sent that payload.
    pub keyframe: bool,
    /// Luma x of a region update. Zero with a zero damage size means the
    /// payload is a full frame.
    pub damage_x: u32,
    /// Luma y of a region update.
    pub damage_y: u32,
    /// Width of a region update. Zero means the payload is a full frame.
    pub damage_width: u32,
    /// Height of a region update.
    pub damage_height: u32,
    /// The picture did not change. The payload is empty.
    pub unchanged: bool,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for VideoFrameHeader {
    fn default() -> Self {
        Self {
            display_id: 0,
            codec: Codec::Unspecified,
            frame_no: 0,
            capture_ts_us: 0,
            keyframe: false,
            damage_x: 0,
            damage_y: 0,
            damage_width: 0,
            damage_height: 0,
            unchanged: false,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for VideoFrameHeader {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.display_id);
        wire::write_i32(out, 2, self.codec.as_i32());
        wire::write_u64(out, 3, self.frame_no);
        wire::write_u64(out, 4, self.capture_ts_us);
        wire::write_bool(out, 5, self.keyframe);
        wire::write_u32(out, 6, self.damage_x);
        wire::write_u32(out, 7, self.damage_y);
        wire::write_u32(out, 8, self.damage_width);
        wire::write_u32(out, 9, self.damage_height);
        wire::write_bool(out, 10, self.unchanged);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            display_id: fields.take_u32(1)?,
            codec: Codec::from_i32(fields.take_i32(2)?),
            frame_no: fields.take_u64(3)?,
            capture_ts_us: fields.take_u64(4)?,
            keyframe: fields.take_bool(5)?,
            damage_x: fields.take_u32(6)?,
            damage_y: fields.take_u32(7)?,
            damage_width: fields.take_u32(8)?,
            damage_height: fields.take_u32(9)?,
            unchanged: fields.take_bool(10)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Receiver feedback, sent about ten times a second.
#[derive(Clone, Debug)]
pub struct Feedback {
    /// Received kilobits per second.
    pub recv_kbps: u32,
    /// Loss fraction, from 0.0 to 1.0.
    pub loss: f32,
    /// Frame delay in milliseconds.
    pub frame_delay_ms: u32,
    /// Last frame number the receiver decoded.
    pub last_frame_no: u64,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl PartialEq for Feedback {
    fn eq(&self, other: &Self) -> bool {
        self.recv_kbps == other.recv_kbps
            && self.loss.to_bits() == other.loss.to_bits()
            && self.frame_delay_ms == other.frame_delay_ms
            && self.last_frame_no == other.last_frame_no
            && self.unknown == other.unknown
    }
}

impl Default for Feedback {
    fn default() -> Self {
        Self {
            recv_kbps: 0,
            loss: 0.0,
            frame_delay_ms: 0,
            last_frame_no: 0,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for Feedback {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.recv_kbps);
        wire::write_f32(out, 2, self.loss)?;
        wire::write_u32(out, 3, self.frame_delay_ms);
        wire::write_u64(out, 4, self.last_frame_no);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            recv_kbps: fields.take_u32(1)?,
            loss: fields.take_f32(2)?,
            frame_delay_ms: fields.take_u32(3)?,
            last_frame_no: fields.take_u64(4)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Graceful close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Close {
    /// Why the sender is closing.
    pub code: ErrorCode,
    /// Optional non-secret detail.
    pub detail: String,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for Close {
    fn default() -> Self {
        Self {
            code: ErrorCode::Unspecified,
            detail: String::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for Close {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_i32(out, 1, self.code.as_i32());
        wire::write_string(out, 2, &self.detail);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            code: ErrorCode::from_i32(fields.take_i32(1)?),
            detail: fields.take_string(2)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Signed locator. Possession of a valid ticket does not authorize control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviteTicket {
    /// Ticket format version. Writers use 1.
    pub version: u32,
    /// Host Ed25519 public key, 32 bytes.
    pub peer_pk: Vec<u8>,
    /// `ip:port` or `[ipv6]:port` strings. At most eight are meaningful.
    pub direct_addrs: Vec<String>,
    /// Home relay URLs.
    pub relay_urls: Vec<String>,
    /// Unix seconds when the ticket stops being a locator.
    pub expires_at: u64,
    /// Why the ticket was minted.
    pub purpose: InvitePurpose,
    /// Optional PAKE hint. Never a password, and never written to logs.
    pub pake_hint: Vec<u8>,
    /// Ed25519 signature over the ticket with this field cleared.
    pub sig: Vec<u8>,
    /// Fields added by a newer peer. They are covered by the signature.
    pub unknown: UnknownFields,
}

impl Default for InviteTicket {
    fn default() -> Self {
        Self {
            version: 0,
            peer_pk: Vec::new(),
            direct_addrs: Vec::new(),
            relay_urls: Vec::new(),
            expires_at: 0,
            purpose: InvitePurpose::Unspecified,
            pake_hint: Vec::new(),
            sig: Vec::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for InviteTicket {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.version);
        if !self.peer_pk.is_empty() {
            wire::write_bytes(out, 2, &self.peer_pk);
        }
        for addr in &self.direct_addrs {
            wire::write_string(out, 3, addr);
        }
        for url in &self.relay_urls {
            wire::write_string(out, 4, url);
        }
        wire::write_u64(out, 5, self.expires_at);
        wire::write_i32(out, 6, self.purpose.as_i32());
        if !self.pake_hint.is_empty() {
            wire::write_bytes(out, 7, &self.pake_hint);
        }
        if !self.sig.is_empty() {
            wire::write_bytes(out, 8, &self.sig);
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            version: fields.take_u32(1)?,
            peer_pk: fields.take_bytes(2)?,
            direct_addrs: repeated_strings(fields, 3)?,
            relay_urls: repeated_strings(fields, 4)?,
            expires_at: fields.take_u64(5)?,
            purpose: InvitePurpose::from_i32(fields.take_i32(6)?),
            pake_hint: fields.take_bytes(7)?,
            sig: fields.take_bytes(8)?,
            unknown: fields.finish_unknown(),
        })
    }
}

fn repeated_strings(fields: &mut Fields, number: u32) -> Result<Vec<String>, ProtoError> {
    let blobs = fields.take_repeated_bytes(number)?;
    let mut out = Vec::with_capacity(blobs.len());
    for bytes in blobs {
        out.push(String::from_utf8(bytes).map_err(|_| ProtoError::BadUtf8)?);
    }
    Ok(out)
}

/// Address record published to the DHT. The signature lives outside the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DhtAddressRecord {
    /// Direct `ip:port` candidates. Empty when the publisher withheld its IP.
    pub direct_addrs: Vec<String>,
    /// Relay URLs the publisher is willing to use.
    pub relay_urls: Vec<String>,
    /// Protocol major version.
    pub protocol_version: u32,
    /// Unix seconds when the record was signed.
    pub timestamp: u64,
    /// True when `direct_addrs` is allowed to be non-empty.
    pub publish_ip: bool,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for DhtAddressRecord {
    fn default() -> Self {
        Self {
            direct_addrs: Vec::new(),
            relay_urls: Vec::new(),
            protocol_version: 0,
            timestamp: 0,
            publish_ip: false,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for DhtAddressRecord {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        for addr in &self.direct_addrs {
            wire::write_string(out, 1, addr);
        }
        for url in &self.relay_urls {
            wire::write_string(out, 2, url);
        }
        wire::write_u32(out, 3, self.protocol_version);
        wire::write_u64(out, 4, self.timestamp);
        wire::write_bool(out, 5, self.publish_ip);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            direct_addrs: repeated_strings(fields, 1)?,
            relay_urls: repeated_strings(fields, 2)?,
            protocol_version: fields.take_u32(3)?,
            timestamp: fields.take_u64(4)?,
            publish_ip: fields.take_bool(5)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Signed wrapper around an encoded [`DhtAddressRecord`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedDhtRecord {
    /// Ed25519 public key that signed `record`.
    pub signer_pk: Vec<u8>,
    /// Canonical encoding of [`DhtAddressRecord`].
    pub record: Vec<u8>,
    /// Ed25519 signature over `record`.
    pub sig: Vec<u8>,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for SignedDhtRecord {
    fn default() -> Self {
        Self {
            signer_pk: Vec::new(),
            record: Vec::new(),
            sig: Vec::new(),
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for SignedDhtRecord {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        if !self.signer_pk.is_empty() {
            wire::write_bytes(out, 1, &self.signer_pk);
        }
        if !self.record.is_empty() {
            wire::write_bytes(out, 2, &self.record);
        }
        if !self.sig.is_empty() {
            wire::write_bytes(out, 3, &self.sig);
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            signer_pk: fields.take_bytes(1)?,
            record: fields.take_bytes(2)?,
            sig: fields.take_bytes(3)?,
            unknown: fields.finish_unknown(),
        })
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Size limits from the session protocol.
//!
//! Parsers reject buffers above these caps before they allocate further
//! structure. The caps are part of the threat model: a peer cannot make the
//! other side buffer an unbounded control message.

/// Largest accepted control-channel message, including a signed invite ticket.
pub const CONTROL_MAX_BYTES: usize = 64 * 1024;

/// Default clipboard payload cap. Users can lower this; they cannot raise it
/// past the absolute cap below.
pub const CLIPBOARD_DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Absolute clipboard cap, even if configuration asks for more.
pub const CLIPBOARD_ABSOLUTE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Largest accepted encoded video frame, header included.
pub const VIDEO_FRAME_MAX_BYTES: usize = 16 * 1024 * 1024;

/// Direct addresses carried by one invite ticket or DHT record.
pub const MAX_DIRECT_ADDRS: usize = 8;

/// Which payload a caller is about to accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadKind {
    /// Handshake, auth, permissions, invite tickets, DHT records.
    Control,
    /// Clipboard bytes after reassembly.
    Clipboard,
    /// One encoded video frame.
    VideoFrame,
}

/// Reject a payload that is larger than the cap for `kind`.
pub fn check_payload(kind: PayloadKind, len: usize) -> Result<(), crate::ProtoError> {
    let limit = match kind {
        PayloadKind::Control => CONTROL_MAX_BYTES,
        PayloadKind::Clipboard => CLIPBOARD_ABSOLUTE_MAX_BYTES,
        PayloadKind::VideoFrame => VIDEO_FRAME_MAX_BYTES,
    };
    if len > limit {
        return Err(crate::ProtoError::TooLarge { len, limit });
    }
    Ok(())
}

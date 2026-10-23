// SPDX-License-Identifier: AGPL-3.0-or-later

//! One plain-text clipboard update.
//!
//! The bytes do not authorize themselves. The host still has to have granted
//! [`crate::Permission::Clipboard`]. Rich text, images, and files are not
//! this message. The text is capped so the frame stays inside a control-sized
//! buffer; the 64 MiB clipboard ceiling in the limits module is for a later
//! reassembly path.

use crate::error::ProtoError;
use crate::limits::CONTROL_MAX_BYTES;
use crate::wire::{self, Fields, UnknownFields};
use crate::ProtoMessage;

/// Largest UTF-8 clipboard text this message will carry.
pub const CLIPBOARD_TEXT_MAX_BYTES: usize = 60 * 1024;

/// A text clipboard snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardText {
    /// UTF-8 text. Empty clears the peer's text clipboard.
    pub text: String,
    /// Sender-local counter. Receivers apply the latest update they see.
    pub generation: u64,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for ClipboardText {
    fn default() -> Self {
        Self {
            text: String::new(),
            generation: 0,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for ClipboardText {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        if self.text.len() > CLIPBOARD_TEXT_MAX_BYTES {
            return Err(ProtoError::TooLarge {
                len: self.text.len(),
                limit: CLIPBOARD_TEXT_MAX_BYTES,
            });
        }
        wire::write_string(out, 1, &self.text);
        wire::write_u64(out, 2, self.generation);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        let text = fields.take_string(1)?;
        if text.len() > CLIPBOARD_TEXT_MAX_BYTES {
            return Err(ProtoError::TooLarge {
                len: text.len(),
                limit: CLIPBOARD_TEXT_MAX_BYTES,
            });
        }
        Ok(Self {
            text,
            generation: fields.take_u64(2)?,
            unknown: fields.finish_unknown(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_text_round_trips() {
        let message = ClipboardText {
            text: "merhaba".to_string(),
            generation: 4,
            unknown: UnknownFields::new(),
        };
        let bytes = message.to_bytes().expect("encode");
        assert_eq!(ClipboardText::from_bytes(&bytes).expect("decode"), message);
    }

    #[test]
    fn clipboard_text_rejects_an_oversized_body() {
        let message = ClipboardText {
            text: "x".repeat(CLIPBOARD_TEXT_MAX_BYTES + 1),
            generation: 1,
            unknown: UnknownFields::new(),
        };
        assert!(matches!(
            message.to_bytes(),
            Err(ProtoError::TooLarge { .. })
        ));
    }
}

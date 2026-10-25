// SPDX-License-Identifier: AGPL-3.0-or-later

//! One plain-text clipboard update.
//!
//! The bytes do not authorize themselves. The host still has to have granted
//! [`crate::Permission::Clipboard`]. Images and files are not this message.
//! HTML is reduced to text by [`prepare_clipboard_text`] before it is sent.
//! A line whose trimmed text is `cbdesk-secret` refuses the whole snapshot.
//! That mark is not password detection. The text is capped so the frame stays
//! inside a control-sized buffer; the 64 MiB clipboard ceiling in the limits
//! module is for a later reassembly path.

use crate::error::ProtoError;
use crate::limits::CONTROL_MAX_BYTES;
use crate::wire::{self, Fields, UnknownFields};
use crate::ProtoMessage;

/// Largest UTF-8 clipboard text this message will carry.
pub const CLIPBOARD_TEXT_MAX_BYTES: usize = 60 * 1024;

/// The whole line that keeps a snapshot on this machine.
pub const CLIPBOARD_SECRET_MARK: &str = "cbdesk-secret";

/// Reduce `text` to plain text that may cross the session.
///
/// HTML tags are dropped and a few entities are decoded. A snapshot that
/// contains [`CLIPBOARD_SECRET_MARK`] as a whole line is
/// [`ProtoError::ClipboardSecret`] and must not be sent or written to the
/// other machine. File lists are not accepted here; this function does not
/// invent a file paste.
pub fn prepare_clipboard_text(text: &str) -> Result<String, ProtoError> {
    if marked_secret(text) {
        return Err(ProtoError::ClipboardSecret);
    }
    let plain = if looks_like_html(text) {
        html_to_text(text)
    } else {
        text.to_string()
    };
    if marked_secret(&plain) {
        return Err(ProtoError::ClipboardSecret);
    }
    if plain.len() > CLIPBOARD_TEXT_MAX_BYTES {
        return Err(ProtoError::TooLarge {
            len: plain.len(),
            limit: CLIPBOARD_TEXT_MAX_BYTES,
        });
    }
    Ok(plain)
}

fn marked_secret(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim().eq_ignore_ascii_case(CLIPBOARD_SECRET_MARK))
}

fn looks_like_html(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(2).any(|pair| {
        pair[0] == b'<' && (pair[1].is_ascii_alphabetic() || pair[1] == b'/' || pair[1] == b'!')
    })
}

fn html_to_text(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            for inner in chars.by_ref() {
                if inner == '>' {
                    break;
                }
                tag.push(inner);
            }
            if tag_breaks(&tag) {
                out.push('\n');
            }
        } else if ch == '&' {
            let mut entity = String::new();
            let mut closed = false;
            while let Some(next) = chars.peek().copied() {
                if next == ';' {
                    chars.next();
                    closed = true;
                    break;
                }
                if entity.len() >= 16 || next.is_whitespace() {
                    break;
                }
                entity.push(next);
                chars.next();
            }
            match (closed, entity.as_str()) {
                (true, "amp") => out.push('&'),
                (true, "lt") => out.push('<'),
                (true, "gt") => out.push('>'),
                (true, "quot") => out.push('"'),
                (true, "apos" | "#39") => out.push('\''),
                (true, "nbsp") => out.push(' '),
                (true, _) => {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
                (false, _) => {
                    out.push('&');
                    out.push_str(&entity);
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn tag_breaks(tag: &str) -> bool {
    let name = tag
        .trim()
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");
    let name = name.trim_end_matches('/');
    matches!(
        name.to_ascii_lowercase().as_str(),
        "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3"
    )
}

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
    fn html_becomes_text_and_a_secret_line_is_not_shared() {
        let plain = prepare_clipboard_text("<p>hello&nbsp;<b>there</b></p>").expect("html");
        assert!(plain.contains("hello"));
        assert!(plain.contains("there"));
        assert!(!plain.contains("<p>"));
        assert!(!plain.contains("&nbsp;"));
        assert!(matches!(
            prepare_clipboard_text("keep\ncbdesk-secret\n"),
            Err(ProtoError::ClipboardSecret)
        ));
        assert!(matches!(
            prepare_clipboard_text("<div>cbdesk-secret</div>"),
            Err(ProtoError::ClipboardSecret)
        ));
        assert_eq!(prepare_clipboard_text("plain").expect("plain"), "plain");
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

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Text clipboard sync.
//!
//! [`ClipSync`] drops echoes of a value this process just applied. The
//! operating-system clipboard is touched only when a session asked for it.
//! Tests use [`ClipSync`] and do not call [`OsClipboard`].

use arboard::Clipboard;
use cbdesk_proto::{prepare_clipboard_text, ProtoError};

use crate::error::CliError;

/// Plain text that may cross the session.
///
/// `Ok(None)` is a `cbdesk-secret` snapshot: do not send it and do not write
/// it to the other machine. HTML is reduced to text. File paste is not this
/// function.
pub fn outgoing_clipboard(text: &str) -> Result<Option<String>, CliError> {
    match prepare_clipboard_text(text) {
        Ok(text) => Ok(Some(text)),
        Err(ProtoError::ClipboardSecret) => Ok(None),
        Err(err) => Err(CliError::new(err.to_string())),
    }
}

/// Remember the last text each direction applied.
#[derive(Clone, Debug, Default)]
pub struct ClipSync {
    last_remote: Option<String>,
    last_local: Option<String>,
    generation: u64,
    os_broken: bool,
}

impl ClipSync {
    /// No text seen yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record text that arrived from the peer. False when it is a repeat.
    pub fn note_remote(&mut self, text: &str) -> bool {
        if self.last_remote.as_deref() == Some(text) {
            return false;
        }
        self.last_remote = Some(text.to_string());
        true
    }

    /// A local change worth sending, with the next generation.
    pub fn local_change(&mut self, text: &str) -> Option<u64> {
        if self.last_remote.as_deref() == Some(text) || self.last_local.as_deref() == Some(text) {
            return None;
        }
        self.last_local = Some(text.to_string());
        self.generation = self.generation.saturating_add(1);
        Some(self.generation)
    }

    /// The OS clipboard already failed once in this process.
    pub fn os_broken(&self) -> bool {
        self.os_broken
    }

    /// Remember that the OS clipboard cannot be used.
    pub fn mark_os_broken(&mut self) {
        self.os_broken = true;
    }
}

/// The operating-system text clipboard.
pub struct OsClipboard;

impl OsClipboard {
    /// Current text, or `None` when the clipboard has no text.
    pub fn get() -> Result<Option<String>, CliError> {
        let mut board = Clipboard::new().map_err(|err| CliError::new(err.to_string()))?;
        match board.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(err) => Err(CliError::new(err.to_string())),
        }
    }

    /// Replace the text clipboard.
    pub fn set(text: &str) -> Result<(), CliError> {
        let mut board = Clipboard::new().map_err(|err| CliError::new(err.to_string()))?;
        board
            .set_text(text)
            .map_err(|err| CliError::new(err.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_of_remote_text_is_not_sent_again() {
        let mut sync = ClipSync::new();
        assert!(sync.note_remote("merhaba"));
        assert!(!sync.note_remote("merhaba"));
        assert_eq!(sync.local_change("merhaba"), None);
        assert_eq!(sync.local_change("yeni"), Some(1));
        assert_eq!(sync.local_change("yeni"), None);
    }
}

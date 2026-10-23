// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Hand-written Flutter API.
//!
//! The manual clock and the test-only consent arm are not on this surface.
//! A short code still does not authenticate anyone. The session indicator
//! still has no off switch.

use std::path::Path;

use cbdesk_desk::{Desk, DeskError, DeskSnapshot};

/// One desktop process. Dart holds this opaque handle.
#[flutter_rust_bridge::frb(opaque)]
pub struct Desktop {
    desk: Desk,
}

impl Desktop {
    /// Load or create `dir` and listen for a controller.
    pub fn open(dir: String) -> Result<Desktop, String> {
        Desk::open(Path::new(&dir))
            .map(|desk| Desktop { desk })
            .map_err(as_code)
    }

    /// Current screen data.
    pub fn snapshot(&self) -> Result<UiSnapshot, String> {
        self.desk.snapshot().map(UiSnapshot::from).map_err(as_code)
    }

    /// `tr` or `en`. The short code already on screen stays as it was.
    pub fn set_locale(&self, locale: String) -> Result<(), String> {
        self.desk.set_locale(&locale).map_err(as_code)
    }

    /// Store an operator relay URL. Empty clears it. This process does not
    /// start a relay.
    pub fn set_relay(&self, relay: String) -> Result<(), String> {
        self.desk.set_relay(&relay).map_err(as_code)
    }

    /// Re-read the default microphone and speaker names.
    pub fn refresh_audio(&self) -> Result<(), String> {
        self.desk.refresh_audio().map_err(as_code)
    }

    /// Remember a contact. `device_id` must be a real device id.
    pub fn add_contact(&self, name: String, device_id: String) -> Result<(), String> {
        self.desk.add_contact(&name, &device_id).map_err(as_code)
    }

    /// Remove a contact. Unknown ids are left as they were.
    pub fn remove_contact(&self, device_id: String) -> Result<(), String> {
        self.desk.remove_contact(&device_id).map_err(as_code)
    }

    /// Jail `name` under the transfer directory and remember it.
    pub fn stage_file(&self, name: String) -> Result<(), String> {
        self.desk.stage_file(&name).map_err(as_code)
    }

    /// Dial `text`. A short code is refused. A ticket is only a locator.
    pub fn connect(&self, text: String) -> Result<(), String> {
        self.desk.connect(&text).map_err(as_code)
    }

    /// Approve the current prompt. Too early, and the prompt stays up.
    pub fn approve(&self, perms: Vec<String>, explicit_high_risk: bool) -> Result<(), String> {
        self.desk
            .approve(&perms, explicit_high_risk)
            .map_err(as_code)
    }

    /// Deny the prompt. The indicator goes away.
    pub fn reject(&self) -> Result<(), String> {
        self.desk.reject().map_err(as_code)
    }

    /// Reject this prompt and refuse the same device id later.
    pub fn block(&self) -> Result<(), String> {
        self.desk.block().map_err(as_code)
    }

    /// End the prompt or the active session. This is the indicator's Kes button.
    pub fn disconnect(&self) -> Result<(), String> {
        self.desk.disconnect().map_err(as_code)
    }
}

/// Permissions the consent dialog may offer.
pub fn permission_offers() -> Vec<UiPermission> {
    cbdesk_desk::permission_offers()
        .into_iter()
        .map(|row| UiPermission {
            name: row.name,
            high_risk: row.high_risk,
        })
        .collect()
}

/// Called once when the Dart isolate starts.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// One consent checkbox.
pub struct UiPermission {
    /// Protocol name, such as `view` or `file_write`.
    pub name: String,
    /// True when a first contact must opt in before this can be granted.
    pub high_risk: bool,
}

/// One address-book row.
pub struct UiContact {
    /// Name the person typed.
    pub name: String,
    /// Canonical device id.
    pub device_id: String,
}

/// What the shell draws.
pub struct UiSnapshot {
    /// Full device id.
    pub device_id: String,
    /// Display fingerprint. Not enough to authenticate a peer.
    pub short_fingerprint: String,
    /// `NNN-WORD-WORD`. Not a credential in this build.
    pub short_code: String,
    /// Always false until a PAKE exists.
    pub short_code_authenticates: bool,
    /// Signed invite ticket, hex. A locator, not a grant.
    pub ticket_hex: String,
    /// `tr` or `en`.
    pub locale: String,
    /// `idle`, `awaiting`, `active`, or `closed`.
    pub phase: String,
    /// True while consent is pending or the session is active.
    pub indicator_visible: bool,
    /// Always false.
    pub indicator_can_be_disabled: bool,
    /// False until the first-contact delay has elapsed.
    pub accept_enabled: bool,
    /// True while the consent dialog should show the fraud warning.
    pub fraud_warning: bool,
    /// True when this prompt is a first contact.
    pub first_contact: bool,
    /// Peer device id, when a prompt or a session exists.
    pub peer_id: String,
    /// Peer fingerprint.
    pub fingerprint: String,
    /// Permissions granted on the active session.
    pub granted: Vec<String>,
    /// Decoded remote picture, tightly packed RGBA.
    pub frame_rgba: Vec<u8>,
    /// Frame width. Zero when no picture has been decoded.
    pub frame_width: u32,
    /// Frame height.
    pub frame_height: u32,
    /// True when the shown picture was decoded from an inter frame.
    pub frame_is_inter: bool,
    /// File names inside the transfer directory.
    pub files: Vec<String>,
    /// Address book.
    pub contacts: Vec<UiContact>,
    /// Names staged for a later send.
    pub staged: Vec<String>,
    /// Default input device, or `unavailable`.
    pub audio_input: String,
    /// Default output device, or `unavailable`.
    pub audio_output: String,
    /// Operator relay URL.
    pub relay: String,
    /// Last local error. Empty when the last action succeeded.
    pub error: String,
    /// Device ids this desk will not prompt for.
    pub blocked: Vec<String>,
}

impl From<DeskSnapshot> for UiSnapshot {
    fn from(snap: DeskSnapshot) -> Self {
        Self {
            device_id: snap.device_id,
            short_fingerprint: snap.short_fingerprint,
            short_code: snap.short_code,
            short_code_authenticates: snap.short_code_authenticates,
            ticket_hex: snap.ticket_hex,
            locale: snap.locale,
            phase: snap.phase,
            indicator_visible: snap.indicator_visible,
            indicator_can_be_disabled: snap.indicator_can_be_disabled,
            accept_enabled: snap.accept_enabled,
            fraud_warning: snap.fraud_warning,
            first_contact: snap.first_contact,
            peer_id: snap.peer_id,
            fingerprint: snap.fingerprint,
            granted: snap.granted,
            frame_rgba: snap.frame_rgba,
            frame_width: snap.frame_width,
            frame_height: snap.frame_height,
            frame_is_inter: snap.frame_is_inter,
            files: snap.files,
            contacts: snap
                .contacts
                .into_iter()
                .map(|row| UiContact {
                    name: row.name,
                    device_id: row.device_id,
                })
                .collect(),
            staged: snap.staged,
            audio_input: snap.audio_input,
            audio_output: snap.audio_output,
            relay: snap.relay,
            error: snap.error,
            blocked: snap.blocked,
        }
    }
}

fn as_code(err: DeskError) -> String {
    match err {
        DeskError::ShortCodeDoesNotAuthenticate => "short_code".into(),
        DeskError::Blocked => "blocked".into(),
        DeskError::NoPrompt => "no_prompt".into(),
        DeskError::LooseSeed => "loose_seed".into(),
        DeskError::Message(text) => format!("message:{text}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{permission_offers, Desktop};

    #[test]
    fn the_bridge_refuses_a_short_code_and_keeps_the_indicator_locked() {
        let dir = tempfile::tempdir().unwrap();
        let desk = Desktop::open(dir.path().to_string_lossy().into_owned()).unwrap();
        let snap = desk.snapshot().unwrap();
        assert_eq!(snap.device_id.len(), 52);
        assert!(!snap.short_code_authenticates);
        assert!(!snap.indicator_can_be_disabled);
        assert!(!snap.indicator_visible);
        let err = desk.connect(snap.short_code.clone()).unwrap_err();
        assert_eq!(err, "short_code");
        let offers = permission_offers();
        assert!(offers
            .iter()
            .any(|row| row.name == "file_write" && row.high_risk));
        assert!(offers
            .iter()
            .any(|row| row.name == "audio" && !row.high_risk));
    }
}

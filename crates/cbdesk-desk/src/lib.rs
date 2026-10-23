// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Desktop shell state in front of the Rust core.
//!
//! The home screen shows a device id, a short fingerprint, and a short code.
//! The short code does not authenticate anyone (ADR-008). A signed ticket is
//! a locator. Consent still waits three seconds on first contact, and the
//! session indicator has no off switch. The remote view is a picture this
//! process decoded from a motion-compensated VP8 inter frame.

mod error;
mod hexutil;
mod live;
mod preview;
mod state;

use std::path::Path;
use std::sync::{Arc, Mutex};

use cbdesk_net::QuicEndpoint;
use cbdesk_proto::ProtoMessage;

pub use error::DeskError;
pub use state::{Contact, DeskSnapshot, Settings};

/// One consent checkbox. `high_risk` is true for file write and the other
/// first-contact gates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionOffer {
    /// Protocol name, such as `view` or `file_write`.
    pub name: String,
    /// True when a first contact must opt in before this can be granted.
    pub high_risk: bool,
}

/// The permissions the consent dialog may offer.
pub fn permission_offers() -> Vec<PermissionOffer> {
    state::catalog()
        .into_iter()
        .map(|(name, perm)| PermissionOffer {
            name: name.to_string(),
            high_risk: perm.is_high_risk(),
        })
        .collect()
}

use state::Inner;

/// One desktop process: identity, listener, and the screens' state.
#[derive(Clone)]
pub struct Desk {
    inner: Arc<Mutex<Inner>>,
    runtime: Arc<tokio::runtime::Runtime>,
    endpoint: QuicEndpoint,
}

impl Desk {
    /// Load or create `dir` and listen for a controller.
    pub fn open(dir: &Path) -> Result<Self, DeskError> {
        Self::open_at(dir, false)
    }

    /// Like [`Self::open`], with a clock the test sets.
    pub fn open_manual(dir: &Path) -> Result<Self, DeskError> {
        Self::open_at(dir, true)
    }

    fn open_at(dir: &Path, manual: bool) -> Result<Self, DeskError> {
        let inner = Arc::new(Mutex::new(state::load_dir(dir, manual)?));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let endpoint = runtime
            .block_on(live::bind_endpoint(&inner))
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let desk = Self {
            inner,
            runtime: Arc::new(runtime),
            endpoint,
        };
        live::listen(
            desk.runtime.handle().clone(),
            desk.inner.clone(),
            desk.endpoint.clone(),
        );
        Ok(desk)
    }

    /// Current screen data.
    pub fn snapshot(&self) -> Result<DeskSnapshot, DeskError> {
        let guard = lock(&self.inner)?;
        Ok(state::snapshot(&guard, state::clock_ms(&guard)))
    }

    /// Move the manual clock. The Flutter bridge does not call this.
    pub fn set_clock_ms(&self, now_ms: u64) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        guard.manual = true;
        guard.manual_ms = now_ms;
        Ok(())
    }

    /// `tr` or `en`. The short code already on screen is left as it was.
    pub fn set_locale(&self, locale: &str) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        state::apply_locale(&mut guard, locale)
    }

    /// Store an operator relay URL. Empty clears it. This process does not
    /// start a relay.
    pub fn set_relay(&self, relay: &str) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        state::apply_relay(&mut guard, relay)
    }

    /// Re-read the default devices.
    pub fn refresh_audio(&self) -> Result<(), DeskError> {
        let names = cbdesk_audio::probe_defaults();
        let mut guard = lock(&self.inner)?;
        guard.audio_input = names.input.unwrap_or_else(|| "unavailable".into());
        guard.audio_output = names.output.unwrap_or_else(|| "unavailable".into());
        Ok(())
    }

    /// Remember a contact. `device_id` must be a real device id.
    pub fn add_contact(&self, name: &str, device_id: &str) -> Result<(), DeskError> {
        let id = state::parse_device(device_id)?;
        if name.trim().is_empty() {
            return Err(DeskError::Message("contact name is empty".into()));
        }
        let mut guard = lock(&self.inner)?;
        guard.contacts.retain(|row| row.device_id != id.to_z32());
        guard.contacts.push(Contact {
            name: name.trim().to_string(),
            device_id: id.to_z32(),
        });
        state::save_contacts(&guard)
    }

    /// Remove a contact. Unknown ids are left as they were.
    pub fn remove_contact(&self, device_id: &str) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        guard.contacts.retain(|row| row.device_id != device_id);
        state::save_contacts(&guard)
    }

    /// Jail `name` under the transfer directory and remember it.
    pub fn stage_file(&self, name: &str) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        let root = state::files_root(&guard);
        cbdesk_fs::resolve(&root, name).map_err(|err| DeskError::Message(err.to_string()))?;
        if !guard.staged.iter().any(|staged| staged == name) {
            guard.staged.push(name.to_string());
        }
        Ok(())
    }

    /// Dial `text`. A short code is refused. A ticket is only a locator;
    /// the host still has to approve.
    pub fn connect(&self, text: &str) -> Result<(), DeskError> {
        if short_code_text(text) {
            return Err(DeskError::ShortCodeDoesNotAuthenticate);
        }
        let bytes = hexutil::decode(text)?;
        let ticket = cbdesk_proto::InviteTicket::from_bytes(&bytes)
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let (tx, rx) = std::sync::mpsc::channel();
        let inner = Arc::clone(&self.inner);
        let endpoint = self.endpoint.clone();
        self.runtime.spawn(async move {
            let result = live::dial_and_show(&inner, &endpoint, &ticket).await;
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|_| DeskError::Message("connect task ended".into()))?
    }

    /// Approve the current prompt. Too early, and the prompt stays up.
    pub fn approve(&self, perms: &[String], explicit_high_risk: bool) -> Result<(), DeskError> {
        let parsed = state::parse_perms(perms)?;
        let mut guard = lock(&self.inner)?;
        let now = state::clock_ms(&guard);
        guard
            .session
            .approve(now, &parsed, explicit_high_risk)
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let tx = guard.decision.take().ok_or(DeskError::NoPrompt)?;
        drop(guard);
        tx.send(cbdesk_net::ConsentAnswer::Approve {
            now_ms: now,
            perms: parsed,
            explicit_high_risk,
        })
        .map_err(|_| DeskError::NoPrompt)
    }

    /// Deny the prompt. The indicator goes away.
    pub fn reject(&self) -> Result<(), DeskError> {
        let mut guard = lock(&self.inner)?;
        let now = state::clock_ms(&guard);
        guard
            .session
            .reject()
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let tx = guard.decision.take();
        drop(guard);
        if let Some(tx) = tx {
            let _ = tx.send(cbdesk_net::ConsentAnswer::Reject { now_ms: now });
        }
        Ok(())
    }

    /// Reject this prompt and refuse the same device id later.
    pub fn block(&self) -> Result<(), DeskError> {
        let id = {
            let guard = lock(&self.inner)?;
            guard.peer_id.clone()
        };
        if id.is_empty() {
            return Err(DeskError::NoPrompt);
        }
        {
            let mut guard = lock(&self.inner)?;
            state::remember_block(&mut guard, &id)?;
        }
        self.reject()
    }

    /// End the prompt or the active session. This is the indicator's Kes button.
    pub fn disconnect(&self) -> Result<(), DeskError> {
        let _ = self.reject();
        let mut guard = lock(&self.inner)?;
        if let Some(tx) = guard.cancel.take() {
            let _ = tx.send(());
        }
        let _ = guard
            .session
            .close_with(cbdesk_session::CloseReason::LocalClose);
        guard.peer_id.clear();
        guard.fingerprint.clear();
        guard.viewer_live = false;
        Ok(())
    }

    /// Raise a consent prompt without a peer. Tests use this. The shell does not.
    #[cfg(test)]
    pub fn arm_for_test(
        &self,
        device_id: &str,
    ) -> Result<tokio::sync::oneshot::Receiver<cbdesk_net::ConsentAnswer>, DeskError> {
        let peer = state::parse_device(device_id)?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        live::arm(&self.inner, &peer, tx)?;
        Ok(rx)
    }
}

fn lock(inner: &Mutex<Inner>) -> Result<std::sync::MutexGuard<'_, Inner>, DeskError> {
    inner
        .lock()
        .map_err(|_| DeskError::Message("desktop state lock was poisoned".into()))
}

fn short_code_text(text: &str) -> bool {
    use cbdesk_discovery::{parse_code, Language};
    parse_code(text, Language::Tr).is_ok() || parse_code(text, Language::En).is_ok()
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn a_short_code_is_shown_and_does_not_authenticate() {
        let dir = tempfile::tempdir().expect("dir");
        let desk = Desk::open(dir.path()).expect("open");
        let shot = desk.snapshot().expect("shot");
        assert_eq!(shot.device_id.len(), 52);
        assert!(!shot.short_fingerprint.is_empty());
        assert!(!shot.short_code_authenticates);
        assert!(shot.short_code.contains('-'));
        assert!(!shot.ticket_hex.is_empty());
        assert!(!shot.indicator_can_be_disabled);
        assert!(!shot.indicator_visible);
        let err = desk.connect(&shot.short_code).expect_err("code");
        assert_eq!(err, DeskError::ShortCodeDoesNotAuthenticate);
        assert!(desk.connect("not-a-ticket").is_err());
    }

    #[test]
    fn consent_waits_and_the_indicator_has_no_off_switch() {
        let dir = tempfile::tempdir().expect("dir");
        let host = Desk::open_manual(dir.path()).expect("host");
        let peer = Desk::open(&dir.path().join("peer")).expect("peer");
        let peer_id = peer.snapshot().expect("peer").device_id;
        let rx = host.arm_for_test(&peer_id).expect("arm");
        let waiting = host.snapshot().expect("wait");
        assert_eq!(waiting.phase, "awaiting");
        assert!(waiting.fraud_warning);
        assert!(waiting.first_contact);
        assert!(waiting.indicator_visible);
        assert!(!waiting.indicator_can_be_disabled);
        assert!(!waiting.accept_enabled);
        assert!(host.approve(&["view".into()], false).is_err());
        host.set_clock_ms(state::ready_delay_ms()).expect("clock");
        host.approve(&["view".into()], false).expect("approve");
        let answer = rx.blocking_recv().expect("answer");
        assert!(matches!(answer, cbdesk_net::ConsentAnswer::Approve { .. }));
        assert!(host.snapshot().expect("active").indicator_visible);
        host.disconnect().expect("kes");
        assert!(!host.snapshot().expect("gone").indicator_visible);
    }

    #[test]
    fn block_rejects_and_settings_reject_an_unknown_key() {
        let dir = tempfile::tempdir().expect("dir");
        let desk = Desk::open_manual(dir.path()).expect("desk");
        let other = Desk::open(&dir.path().join("other")).expect("other");
        let id = other.snapshot().expect("id").device_id;
        let _rx = desk.arm_for_test(&id).expect("arm");
        desk.block().expect("block");
        assert!(desk.snapshot().expect("shot").blocked.contains(&id));
        assert!(!desk.snapshot().expect("shot").indicator_visible);
        let settings = dir.path().join("settings.json");
        std::fs::write(
            &settings,
            r#"{"locale":"en","relay":"","hide_indicator":true}"#,
        )
        .expect("write");
        let again = Desk::open(&dir.path().join("fresh"));
        assert!(again.is_ok());
        std::fs::write(
            dir.path().join("fresh").join("settings.json"),
            r#"{"locale":"en","relay":"","hide_indicator":true}"#,
        )
        .expect("write");
        // The file is read on open. A second open of that directory must refuse it.
        assert!(Desk::open(&dir.path().join("fresh")).is_err());
    }

    #[test]
    fn address_book_and_the_file_jail_keep_their_promises() {
        let dir = tempfile::tempdir().expect("dir");
        let desk = Desk::open(dir.path()).expect("desk");
        let id = desk.snapshot().expect("id").device_id;
        desk.add_contact("Ada", &id).expect("add");
        assert_eq!(desk.snapshot().expect("book").contacts.len(), 1);
        desk.remove_contact(&id).expect("remove");
        assert!(desk.snapshot().expect("empty").contacts.is_empty());
        assert!(desk.stage_file("../secret").is_err());
        desk.stage_file("note.txt").expect("stage");
        assert_eq!(
            desk.snapshot().expect("staged").staged,
            vec!["note.txt".to_string()]
        );
        desk.set_locale("en").expect("locale");
        desk.set_relay("https://203.0.113.9:443").expect("relay");
        let shot = desk.snapshot().expect("shot");
        assert_eq!(shot.locale, "en");
        assert_eq!(shot.relay, "https://203.0.113.9:443");
        assert!(desk.set_relay("http://203.0.113.9").is_err());
    }

    #[test]
    fn two_desks_show_a_motion_inter_frame_only_after_consent() {
        let dir = tempfile::tempdir().expect("dir");
        let host = Desk::open_manual(&dir.path().join("host")).expect("host");
        let viewer = Desk::open(&dir.path().join("viewer")).expect("viewer");
        let ticket = host.snapshot().expect("ticket").ticket_hex;
        let viewer_for_dial = viewer.clone();
        let dial = thread::spawn(move || viewer_for_dial.connect(&ticket));
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if host.snapshot().expect("poll").phase == "awaiting" {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let waiting = host.snapshot().expect("waiting");
        assert_eq!(waiting.phase, "awaiting", "{}", waiting.error);
        assert!(!waiting.accept_enabled);
        assert!(host.approve(&["view".into()], false).is_err());
        host.set_clock_ms(3_000).expect("clock");
        host.approve(&["view".into()], false).expect("approve");
        dial.join().expect("thread").expect("dial");
        let shot = viewer.snapshot().expect("view");
        assert!(shot.frame_is_inter);
        assert!(shot.indicator_visible);
        assert!(!shot.indicator_can_be_disabled);
        assert_eq!((shot.frame_width, shot.frame_height), (48, 16));
        let moved = mean_rgba(&shot.frame_rgba, 48, 16, 32);
        let left = mean_rgba(&shot.frame_rgba, 48, 0, 16);
        assert!(moved > 180 && left < 80, "moved {moved} left {left}");
    }

    #[cfg(unix)]
    #[test]
    fn a_world_readable_seed_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("dir");
        let desk = Desk::open(dir.path()).expect("open");
        drop(desk);
        let path = dir.path().join("identity.seed");
        let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&path, permissions).expect("mode");
        match Desk::open(dir.path()) {
            Err(DeskError::LooseSeed) => {}
            Err(err) => panic!("{err}"),
            Ok(_) => panic!("a loose seed was accepted"),
        }
    }

    fn mean_rgba(rgba: &[u8], width: u32, start: u32, end: u32) -> u32 {
        let mut sum = 0_u32;
        let mut count = 0_u32;
        let height = 16_u32;
        for row in 0..height {
            for col in start..end {
                let index = ((row * width + col) * 4) as usize;
                sum += u32::from(rgba[index]);
                count += 1;
            }
        }
        sum / count
    }
}

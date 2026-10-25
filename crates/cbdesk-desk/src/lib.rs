// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Desktop shell state in front of the Rust core.
//!
//! The home screen shows a device id, a short fingerprint, and a short code.
//! The two words confirm SPAKE2 pairing. They do not locate a computer. A
//! signed ticket is the locator. Consent still waits three seconds on first
//! contact, and the session indicator has no off switch. After consent the
//! host sends a captured frame when a display is available.

mod error;
mod hexutil;
mod live;
mod preview;
mod session;
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
            .thread_stack_size(8 * 1024 * 1024)
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

    /// Dial `text`. A ticket locates the host. An optional second field is
    /// the short code that confirms pairing. The code alone is refused.
    pub fn connect(&self, text: &str) -> Result<(), DeskError> {
        let (ticket_hex, code) = split_connect(text)?;
        let bytes = hexutil::decode(&ticket_hex)?;
        let ticket = cbdesk_proto::InviteTicket::from_bytes(&bytes)
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let (tx, rx) = std::sync::mpsc::channel();
        let inner = Arc::clone(&self.inner);
        let endpoint = self.endpoint.clone();
        self.runtime.spawn(async move {
            let result = live::dial_and_show(inner, &endpoint, &ticket, code).await;
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|_| DeskError::Message("connect task ended".into()))?
    }

    /// Send a pointer sample to the host. Requires an Input grant.
    pub fn send_pointer(&self, x: i32, y: i32, buttons: u32, wheel: i32) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::Pointer {
            x,
            y,
            buttons,
            wheel,
            reply,
        })?;
        self.wait_reply(rx)
    }

    /// Send a key transition to the host. Requires an Input grant.
    pub fn send_key(&self, keycode: u32, down: bool, modifiers: u32) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::Key {
            keycode,
            down,
            modifiers,
            reply,
        })?;
        self.wait_reply(rx)
    }

    /// Send file bytes. The name is jailed by the receiver. Requires FileWrite.
    pub fn send_file(&self, name: &str, bytes: &[u8]) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::File {
            name: name.to_string(),
            bytes: bytes.to_vec(),
            reply,
        })?;
        self.wait_reply(rx)
    }

    /// Send clipboard text. Empty clears. Requires Clipboard.
    pub fn set_clipboard(&self, text: &str) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::Clipboard {
            text: text.to_string(),
            reply,
        })?;
        self.wait_reply(rx)
    }

    /// Send one audio packet. Requires Audio. This does not open a microphone.
    pub fn send_audio(&self, bytes: &[u8]) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::Audio {
            bytes: bytes.to_vec(),
            reply,
        })?;
        self.wait_reply(rx)
    }

    /// Open or drop the default microphone on the controller.
    pub fn set_mic(&self, enabled: bool) -> Result<(), DeskError> {
        let (reply, rx) = reply_channel();
        self.dispatch(session::Cmd::Mic { enabled, reply })?;
        self.wait_reply(rx)
    }

    /// Queue one RGBA frame for the host to encode. Odd sizes are refused.
    ///
    /// Android calls this after the person accepts the system capture dialog.
    pub fn submit_frame(&self, width: u32, height: u32, rgba: &[u8]) -> Result<(), DeskError> {
        let picture = cbdesk_capture::from_rgba(width, height, rgba)
            .map_err(|err| DeskError::Message(err.to_string()))?;
        let mut guard = lock(&self.inner)?;
        if guard.submitted.len() >= 3 {
            guard.submitted.pop_front();
        }
        guard.submitted.push_back(picture);
        Ok(())
    }

    fn dispatch(&self, cmd: session::Cmd) -> Result<(), DeskError> {
        let guard = lock(&self.inner)?;
        let tx = guard
            .commands
            .as_ref()
            .map(|(_, tx)| tx.clone())
            .ok_or_else(|| DeskError::Message("there is no session".into()))?;
        drop(guard);
        tx.send(cmd)
            .map_err(|_| DeskError::Message("session ended".into()))
    }

    fn wait_reply(
        &self,
        rx: tokio::sync::oneshot::Receiver<Result<(), String>>,
    ) -> Result<(), DeskError> {
        let result = self
            .runtime
            .block_on(rx)
            .map_err(|_| DeskError::Message("session ended".into()))?;
        result.map_err(DeskError::Message)
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

    /// Queue the 48×16 motion pair and stop capturing after it.
    ///
    /// The Flutter bridge does not call this. Tests use it so a session does
    /// not encode a full display.
    pub fn queue_motion_preview(&self) -> Result<(), DeskError> {
        let packets = preview::motion_preview()?;
        let mut guard = lock(&self.inner)?;
        guard.preview.extend(packets);
        guard.preview_hold = true;
        Ok(())
    }

    /// Do not call `capture_picture` when no queued or submitted frame is left.
    ///
    /// The Flutter bridge does not call this.
    pub fn hold_capture(&self) -> Result<(), DeskError> {
        lock(&self.inner)?.preview_hold = true;
        Ok(())
    }

    /// Record input instead of moving the local pointer.
    ///
    /// The Flutter bridge does not call this. Tests use it so a session does
    /// not move the machine's pointer.
    pub fn set_record_input(&self, record: bool) -> Result<(), DeskError> {
        lock(&self.inner)?.record_input = record;
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
    parse_either(text).is_ok()
}

fn parse_either(text: &str) -> Result<cbdesk_discovery::ShortCode, ()> {
    use cbdesk_discovery::{parse_code, Language};
    parse_code(text, Language::Tr)
        .or_else(|_| parse_code(text, Language::En))
        .map_err(|_| ())
}

fn split_connect(text: &str) -> Result<(String, Option<cbdesk_discovery::ShortCode>), DeskError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DeskError::Message("connect text is empty".into()));
    }
    if short_code_text(text) {
        return Err(DeskError::ShortCodeNeedsTicket);
    }
    let mut parts = text.split_whitespace();
    let Some(ticket) = parts.next() else {
        return Err(DeskError::Message("connect text is empty".into()));
    };
    let code_text = parts.next();
    if parts.next().is_some() {
        return Err(DeskError::Message(
            "connect text has too many fields".into(),
        ));
    }
    if short_code_text(ticket) {
        return Err(DeskError::ShortCodeNeedsTicket);
    }
    let code = match code_text {
        Some(code) => Some(
            parse_either(code)
                .map_err(|_| DeskError::Message("pairing code was not recognized".into()))?,
        ),
        None => None,
    };
    Ok((ticket.to_string(), code))
}

type ReplyPair = (
    tokio::sync::oneshot::Sender<Result<(), String>>,
    tokio::sync::oneshot::Receiver<Result<(), String>>,
);

fn reply_channel() -> ReplyPair {
    tokio::sync::oneshot::channel()
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn a_short_code_confirms_pairing_and_does_not_locate() {
        let dir = tempfile::tempdir().expect("dir");
        let desk = Desk::open(dir.path()).expect("open");
        let shot = desk.snapshot().expect("shot");
        assert_eq!(shot.device_id.len(), 52);
        assert!(!shot.short_fingerprint.is_empty());
        assert!(shot.short_code_authenticates);
        assert!(shot.short_code.contains('-'));
        assert!(!shot.ticket_hex.is_empty());
        assert!(!shot.indicator_can_be_disabled);
        assert!(!shot.indicator_visible);
        let err = desk.connect(&shot.short_code).expect_err("code");
        assert_eq!(err, DeskError::ShortCodeNeedsTicket);
        assert!(desk.connect("not-a-ticket").is_err());
        let ticket = cbdesk_proto::InviteTicket::from_bytes(
            &hexutil::decode(&shot.ticket_hex).expect("hex"),
        )
        .expect("ticket");
        assert!(ticket.pake_hint.is_empty());
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
        host.queue_motion_preview().expect("preview");
        let ticket = dial_text(&host);
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

    #[test]
    fn a_wrong_code_does_not_open_the_consent_prompt() {
        let dir = tempfile::tempdir().expect("dir");
        let host = Desk::open(&dir.path().join("host")).expect("host");
        let viewer = Desk::open(&dir.path().join("viewer")).expect("viewer");
        let ticket = host.snapshot().expect("ticket").ticket_hex;
        let err = viewer
            .connect(&format!("{ticket} 000-abandon-ability"))
            .expect_err("code");
        assert!(matches!(err, DeskError::Message(text) if text.contains("pairing")));
        let shot = host.snapshot().expect("host");
        assert_eq!(shot.phase, "idle");
        assert!(!shot.indicator_visible);
    }

    #[test]
    fn the_session_carries_input_clipboard_audio_and_a_file() {
        let dir = tempfile::tempdir().expect("dir");
        let host = Desk::open_manual(&dir.path().join("host")).expect("host");
        let viewer = Desk::open(&dir.path().join("viewer")).expect("viewer");
        host.queue_motion_preview().expect("preview");
        host.set_record_input(true).expect("record");
        let viewer_for_dial = viewer.clone();
        let ticket = dial_text(&host);
        let dial = thread::spawn(move || viewer_for_dial.connect(&ticket));
        wait_awaiting(&host);
        host.set_clock_ms(3_000).expect("clock");
        host.approve(
            &[
                "view".into(),
                "input".into(),
                "clipboard".into(),
                "audio".into(),
                "file_write".into(),
            ],
            true,
        )
        .expect("approve");
        dial.join().expect("thread").expect("dial");
        viewer.send_pointer(12, 34, 0, 0).expect("pointer");
        viewer.send_key(0xE000, true, 0).expect("key");
        viewer.set_clipboard("merhaba").expect("clip");
        viewer.send_audio(&[1, 2, 3, 4]).expect("audio");
        viewer.send_file("note.txt", b"hello").expect("file");
        let shot = wait_host(&host, |shot| {
            shot.last_input == "key 57344 1"
                && shot.clipboard == "merhaba"
                && shot.audio_packets == 1
        });
        assert_eq!(shot.last_input, "key 57344 1");
        assert_eq!(shot.clipboard, "merhaba");
        assert_eq!(shot.audio_packets, 1);
        let stored =
            std::fs::read(dir.path().join("host").join("files").join("note.txt")).expect("file");
        assert_eq!(stored, b"hello");
    }

    #[test]
    fn a_submitted_frame_is_what_the_viewer_decodes() {
        let dir = tempfile::tempdir().expect("dir");
        let host = Desk::open_manual(&dir.path().join("host")).expect("host");
        let viewer = Desk::open(&dir.path().join("viewer")).expect("viewer");
        host.hold_capture().expect("hold");
        host.submit_frame(16, 16, &solid(255, 0, 0)).expect("red");
        host.submit_frame(16, 16, &solid(0, 255, 0)).expect("green");
        let viewer_for_dial = viewer.clone();
        let ticket = dial_text(&host);
        let dial = thread::spawn(move || viewer_for_dial.connect(&ticket));
        wait_awaiting(&host);
        host.set_clock_ms(3_000).expect("clock");
        host.approve(&["view".into()], false).expect("approve");
        dial.join().expect("thread").expect("dial");
        let shot = viewer.snapshot().expect("view");
        assert!(shot.frame_is_inter);
        assert_eq!((shot.frame_width, shot.frame_height), (16, 16));
    }

    fn dial_text(host: &Desk) -> String {
        let shot = host.snapshot().expect("shot");
        format!("{} {}", shot.ticket_hex, shot.short_code)
    }

    fn wait_host(host: &Desk, ready: impl Fn(&DeskSnapshot) -> bool) -> DeskSnapshot {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            let shot = host.snapshot().expect("shot");
            if ready(&shot) {
                return shot;
            }
            thread::sleep(Duration::from_millis(20));
        }
        host.snapshot().expect("shot")
    }

    fn wait_awaiting(host: &Desk) {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if host.snapshot().expect("poll").phase == "awaiting" {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("{}", host.snapshot().expect("phase").error);
    }

    fn solid(r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut rgba = Vec::new();
        for _ in 0..(16 * 16) {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
        rgba
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

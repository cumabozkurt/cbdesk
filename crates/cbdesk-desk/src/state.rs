// SPDX-License-Identifier: AGPL-3.0-or-later

//! Saved desktop state. There is no key that hides the session indicator.

use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cbdesk_capture::Picture;
use cbdesk_crypto::{DeviceId, Identity};
use cbdesk_discovery::{Language, ShortCode};
use cbdesk_session::{ApprovalMode, HostSession, Phase, SecurityConfig, CONSENT_DELAY_MS};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::DeskError;

/// One address-book row. The device id is the full key, not a short code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    /// Name the person typed.
    pub name: String,
    /// Canonical device id.
    pub device_id: String,
}

/// Settings the shell is allowed to store.
///
/// Unknown keys are rejected, so a flag that hides the session indicator
/// cannot be added to this file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// `tr` or `en`.
    pub locale: String,
    /// Operator relay URL. Empty means direct only.
    pub relay: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "tr".into(),
            relay: String::new(),
        }
    }
}

/// What the shell draws. Strings are data; the widgets own translations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeskSnapshot {
    /// Full device id.
    pub device_id: String,
    /// Display fingerprint. Not enough to authenticate a peer.
    pub short_fingerprint: String,
    /// `NNN-WORD-WORD`. The words confirm pairing. They do not locate a peer.
    pub short_code: String,
    /// True when the words can confirm pairing. The ticket is still the locator.
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
    /// True when this peer is not in the trust store. This build always
    /// treats a prompt as first contact.
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
    pub contacts: Vec<Contact>,
    /// Names staged for a later send. The jail has already accepted them.
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
    /// Latest clipboard text this desk applied or received.
    pub clipboard: String,
    /// Opus packets the host has accepted in this process.
    pub audio_packets: u32,
    /// Last input event the host recorded or injected.
    pub last_input: String,
}

pub struct Inner {
    pub dir: PathBuf,
    pub identity: Identity,
    pub short_code: ShortCode,
    pub settings: Settings,
    pub contacts: Vec<Contact>,
    pub blocked: Vec<String>,
    pub staged: Vec<String>,
    pub session: HostSession,
    pub peer_id: String,
    pub fingerprint: String,
    pub ticket_hex: String,
    pub decision: Option<tokio::sync::oneshot::Sender<cbdesk_net::ConsentAnswer>>,
    pub cancel: Option<tokio::sync::oneshot::Sender<()>>,
    pub frame_rgba: Vec<u8>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_is_inter: bool,
    pub audio_input: String,
    pub audio_output: String,
    pub error: String,
    pub viewer_live: bool,
    pub preview: VecDeque<crate::preview::PreviewPacket>,
    pub submitted: VecDeque<Picture>,
    pub preview_hold: bool,
    pub record_input: bool,
    pub last_input: String,
    pub clipboard: String,
    pub audio_packets: u32,
    pub commands: Option<(u64, tokio::sync::mpsc::UnboundedSender<crate::session::Cmd>)>,
    pub session_id: u64,
    pub manual: bool,
    pub manual_ms: u64,
    pub started: Instant,
}

pub fn host_config() -> Result<SecurityConfig, DeskError> {
    let config = SecurityConfig {
        approval: ApprovalMode::Always,
        unattended_enabled: false,
        require_2fa: false,
        quick_support: false,
    };
    config
        .validate()
        .map_err(|err| DeskError::Message(err.to_string()))?;
    Ok(config)
}

pub fn fresh_session() -> Result<HostSession, DeskError> {
    HostSession::new(host_config()?).map_err(|err| DeskError::Message(err.to_string()))
}

pub fn load_dir(dir: &Path, manual: bool) -> Result<Inner, DeskError> {
    fs::create_dir_all(dir).map_err(|err| DeskError::Message(err.to_string()))?;
    fs::create_dir_all(dir.join("files")).map_err(|err| DeskError::Message(err.to_string()))?;
    let settings = read_settings(&dir.join("settings.json"))?;
    let language = language(&settings.locale)?;
    let names = cbdesk_audio::probe_defaults();
    Ok(Inner {
        dir: dir.to_path_buf(),
        identity: load_identity(dir)?,
        short_code: ShortCode::generate(language)
            .map_err(|err| DeskError::Message(err.to_string()))?,
        settings,
        contacts: read_contacts(&dir.join("address-book.json"))?,
        blocked: read_blocked(&dir.join("blocked.json"))?,
        staged: Vec::new(),
        session: fresh_session()?,
        peer_id: String::new(),
        fingerprint: String::new(),
        ticket_hex: String::new(),
        decision: None,
        cancel: None,
        frame_rgba: Vec::new(),
        frame_width: 0,
        frame_height: 0,
        frame_is_inter: false,
        audio_input: names.input.unwrap_or_else(|| "unavailable".into()),
        audio_output: names.output.unwrap_or_else(|| "unavailable".into()),
        error: String::new(),
        viewer_live: false,
        preview: VecDeque::new(),
        submitted: VecDeque::new(),
        preview_hold: false,
        record_input: false,
        last_input: String::new(),
        clipboard: String::new(),
        audio_packets: 0,
        commands: None,
        session_id: 0,
        manual,
        manual_ms: 0,
        started: Instant::now(),
    })
}

pub fn snapshot(inner: &Inner, now_ms: u64) -> DeskSnapshot {
    let (mut phase, accept, fraud, first, granted) = phase_bits(&inner.session, now_ms);
    let mut indicator = inner.session.indicator_visible();
    if inner.viewer_live {
        phase = "active";
        indicator = true;
    }
    DeskSnapshot {
        device_id: inner.identity.device_id().to_z32(),
        short_fingerprint: inner.identity.device_id().short_fingerprint().to_string(),
        short_code: inner.short_code.format(),
        short_code_authenticates: true,
        ticket_hex: inner.ticket_hex.clone(),
        locale: inner.settings.locale.clone(),
        phase: phase.into(),
        indicator_visible: indicator,
        indicator_can_be_disabled: HostSession::indicator_can_be_disabled(),
        accept_enabled: accept,
        fraud_warning: fraud,
        first_contact: first,
        peer_id: inner.peer_id.clone(),
        fingerprint: inner.fingerprint.clone(),
        granted,
        frame_rgba: inner.frame_rgba.clone(),
        frame_width: inner.frame_width,
        frame_height: inner.frame_height,
        frame_is_inter: inner.frame_is_inter,
        files: list_files(&inner.dir.join("files")),
        contacts: inner.contacts.clone(),
        staged: inner.staged.clone(),
        audio_input: inner.audio_input.clone(),
        audio_output: inner.audio_output.clone(),
        relay: inner.settings.relay.clone(),
        error: inner.error.clone(),
        blocked: inner.blocked.clone(),
        clipboard: inner.clipboard.clone(),
        audio_packets: inner.audio_packets,
        last_input: inner.last_input.clone(),
    }
}

fn phase_bits(session: &HostSession, now_ms: u64) -> (&'static str, bool, bool, bool, Vec<String>) {
    match session.phase() {
        Phase::Idle => ("idle", false, false, false, Vec::new()),
        Phase::Authenticating { .. } => ("authenticating", false, false, false, Vec::new()),
        Phase::AwaitingConsent {
            ready_at_ms,
            first_contact,
            ..
        } => (
            "awaiting",
            now_ms >= *ready_at_ms,
            true,
            *first_contact,
            Vec::new(),
        ),
        Phase::Active { perms, .. } => ("active", false, false, false, perm_names(perms)),
        Phase::Closed { .. } => ("closed", false, false, false, Vec::new()),
    }
}

fn perm_names(perms: &cbdesk_session::PermissionSet) -> Vec<String> {
    let mut names = Vec::new();
    for (name, perm) in catalog() {
        if perms.contains(perm) {
            names.push(name.into());
        }
    }
    names
}

pub fn catalog() -> [(&'static str, cbdesk_proto::Permission); 6] {
    use cbdesk_proto::Permission;
    [
        ("view", Permission::View),
        ("input", Permission::Input),
        ("clipboard", Permission::Clipboard),
        ("audio", Permission::Audio),
        ("file_read", Permission::FileRead),
        ("file_write", Permission::FileWrite),
    ]
}

pub fn parse_perms(names: &[String]) -> Result<Vec<cbdesk_proto::Permission>, DeskError> {
    let mut perms = Vec::new();
    for name in names {
        let perm = catalog()
            .into_iter()
            .find(|(label, _)| *label == name)
            .map(|(_, perm)| perm)
            .ok_or_else(|| DeskError::Message(format!("unknown permission {name}")))?;
        perms.push(perm);
    }
    Ok(perms)
}

pub fn clock_ms(inner: &Inner) -> u64 {
    if inner.manual {
        inner.manual_ms
    } else {
        u64::try_from(inner.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

pub fn ready_delay_ms() -> u64 {
    CONSENT_DELAY_MS
}

fn language(locale: &str) -> Result<Language, DeskError> {
    match locale {
        "tr" => Ok(Language::Tr),
        "en" => Ok(Language::En),
        _ => Err(DeskError::Message("locale must be tr or en".into())),
    }
}

pub fn apply_locale(inner: &mut Inner, locale: &str) -> Result<(), DeskError> {
    language(locale)?;
    inner.settings.locale = locale.to_string();
    write_json(&inner.dir.join("settings.json"), &inner.settings)
}

pub fn apply_relay(inner: &mut Inner, relay: &str) -> Result<(), DeskError> {
    if !relay.is_empty() && !relay.starts_with("https://") && !relay.starts_with("relay://") {
        return Err(DeskError::Message(
            "relay URL needs an https or relay scheme".into(),
        ));
    }
    inner.settings.relay = relay.to_string();
    write_json(&inner.dir.join("settings.json"), &inner.settings)
}

pub fn remember_block(inner: &mut Inner, device_id: &str) -> Result<(), DeskError> {
    if !inner.blocked.iter().any(|id| id == device_id) {
        inner.blocked.push(device_id.to_string());
    }
    write_json(
        &inner.dir.join("blocked.json"),
        &BlockFile {
            ids: inner.blocked.clone(),
        },
    )
}

fn list_files(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(read) = fs::read_dir(root) else {
        return names;
    };
    for entry in read.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if entry.path().is_file() {
            names.push(name);
        }
    }
    names.sort();
    names
}

fn load_identity(dir: &Path) -> Result<Identity, DeskError> {
    let path = dir.join("identity.seed");
    if path.exists() {
        refuse_loose(&path)?;
        let bytes = fs::read(&path).map_err(|err| DeskError::Message(err.to_string()))?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| DeskError::Message("identity file has the wrong length".into()))?;
        let identity =
            Identity::from_seed(&seed).map_err(|err| DeskError::Message(err.to_string()))?;
        let mut seed = seed;
        seed.zeroize();
        return Ok(identity);
    }
    let identity = Identity::generate().map_err(|err| DeskError::Message(err.to_string()))?;
    let mut seed = identity.signing_seed();
    write_seed(&path, &seed)?;
    seed.zeroize();
    Ok(identity)
}

fn refuse_loose(path: &Path) -> Result<(), DeskError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)
            .map_err(|err| DeskError::Message(err.to_string()))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(DeskError::LooseSeed);
        }
    }
    let _ = path;
    Ok(())
}

fn write_seed(path: &Path, seed: &[u8]) -> Result<(), DeskError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| DeskError::Message(err.to_string()))?;
    file.write_all(seed)
        .map_err(|err| DeskError::Message(err.to_string()))
}

fn read_settings(path: &Path) -> Result<Settings, DeskError> {
    if !path.exists() {
        let settings = Settings::default();
        write_json(path, &settings)?;
        return Ok(settings);
    }
    let text = fs::read_to_string(path).map_err(|err| DeskError::Message(err.to_string()))?;
    let settings: Settings = serde_json::from_str(&text)
        .map_err(|_| DeskError::Message("settings file was rejected".into()))?;
    language(&settings.locale)?;
    if !settings.relay.is_empty()
        && !settings.relay.starts_with("https://")
        && !settings.relay.starts_with("relay://")
    {
        return Err(DeskError::Message("settings relay URL is invalid".into()));
    }
    Ok(settings)
}

fn read_contacts(path: &Path) -> Result<Vec<Contact>, DeskError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|err| DeskError::Message(err.to_string()))?;
    serde_json::from_str(&text).map_err(|_| DeskError::Message("address book was rejected".into()))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockFile {
    ids: Vec<String>,
}

fn read_blocked(path: &Path) -> Result<Vec<String>, DeskError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|err| DeskError::Message(err.to_string()))?;
    let file: BlockFile = serde_json::from_str(&text)
        .map_err(|_| DeskError::Message("block list was rejected".into()))?;
    Ok(file.ids)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), DeskError> {
    let text =
        serde_json::to_string_pretty(value).map_err(|err| DeskError::Message(err.to_string()))?;
    fs::write(path, text).map_err(|err| DeskError::Message(err.to_string()))
}

pub fn save_contacts(inner: &Inner) -> Result<(), DeskError> {
    write_json(&inner.dir.join("address-book.json"), &inner.contacts)
}

pub fn files_root(inner: &Inner) -> PathBuf {
    inner.dir.join("files")
}

/// Used by tests that want a device id without opening a desk.
pub fn parse_device(text: &str) -> Result<DeviceId, DeskError> {
    DeviceId::from_z32(text).map_err(|err| DeskError::Message(err.to_string()))
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Argument parsing for the two subcommands. No capture happens here.

use std::net::SocketAddr;
use std::path::PathBuf;

use cbdesk_proto::Permission;

use crate::error::CliError;

/// Which command to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Listen, print a ticket, and maybe grant.
    Host(HostArgs),
    /// Dial a ticket or browse the LAN, then show frames.
    View(ViewArgs),
    /// Print a local network diagnosis. This does not start a session.
    NetCheck(crate::check::NetCheckArgs),
}

/// Host options. `--approve` is the person's decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostArgs {
    /// UDP socket for QUIC.
    pub bind: SocketAddr,
    /// Permissions to grant after the first-contact delay. Empty rejects.
    pub approve: Vec<Permission>,
    /// The person opted into a high-risk permission on first contact.
    pub allow_high_risk: bool,
    /// Capture the operating-system display instead of the barcode.
    pub screen: bool,
    /// How many frames to send after the grant. Zero streams until the viewer closes.
    pub frames: u32,
    /// Announce on the LAN. The operator asked for this; the profile is private.
    pub lan: bool,
    /// Relay URLs this process may use. Empty stays direct.
    pub relays: Vec<String>,
    /// Directory that jailed file transfers may touch.
    pub files_root: Option<PathBuf>,
    /// Send a synthetic Opus tone when Audio is granted.
    pub tone: bool,
    /// Capture the default microphone and send Opus when Audio is granted.
    pub mic: bool,
    /// Play received Opus on the default speaker.
    pub speaker: bool,
}

/// Viewer options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewArgs {
    /// Hex-encoded invite ticket. Absent when [`Self::browse`] is set.
    pub ticket_hex: Option<String>,
    /// Find a host on the LAN instead of dialing a ticket.
    pub browse: bool,
    /// Where to write the first frame as a PPM. Optional when a window is open.
    pub ppm: Option<PathBuf>,
    /// Open a window and keep reading until it closes.
    pub window: bool,
    /// Optional pointer sample to send after the first frame, if Input was granted.
    pub pointer: Option<(i32, i32)>,
    /// Optional key name to send after the first frame, if Input was granted.
    pub key: Option<String>,
    /// Upload this file after the grant, if FileWrite was granted.
    pub put: Option<PathBuf>,
    /// Request Audio and print the length of packets the host sends.
    pub audio: bool,
    /// Send the default microphone to the host when Audio is granted.
    pub mic: bool,
    /// Play received Opus on the default speaker.
    pub speaker: bool,
    /// Relay URLs this process may use. A ticket cannot add one.
    pub relays: Vec<String>,
}

/// Parse `args`, which does not include the program name.
pub fn parse(args: &[String]) -> Result<Command, CliError> {
    let Some(command) = args.first() else {
        return Err(usage());
    };
    match command.as_str() {
        "host" => Ok(Command::Host(parse_host(&args[1..])?)),
        "view" => Ok(Command::View(parse_view(&args[1..])?)),
        "net-check" | "doctor" => Ok(Command::NetCheck(parse_net_check(&args[1..])?)),
        _ => Err(usage()),
    }
}

fn usage() -> CliError {
    CliError::new(
        "usage: cbdesk host --bind ADDR (--synthetic|--screen) [--approve view,input] \
         [--frames N] [--lan] [--relay URL] [--files-root DIR] [--tone|--mic] [--speaker]\n       \
         cbdesk view (--ticket HEX|--browse) (--ppm FILE|--window|--put FILE|--audio|--mic|--speaker) \
         [--pointer X,Y] [--key NAME] [--relay URL]\n       \
         cbdesk net-check [--json] [--anonymous] [--loopback]\n       \
         cbdesk doctor [--json] [--anonymous] [--loopback]",
    )
}

struct HostDraft {
    bind: SocketAddr,
    approve: Vec<Permission>,
    allow_high_risk: bool,
    screen: Option<bool>,
    frames: u32,
    lan: bool,
    relays: Vec<String>,
    files_root: Option<PathBuf>,
    tone: bool,
    mic: bool,
    speaker: bool,
}

fn parse_net_check(args: &[String]) -> Result<crate::check::NetCheckArgs, CliError> {
    let mut parsed = crate::check::NetCheckArgs {
        json: false,
        anonymous: false,
        loopback: false,
    };
    for flag in args {
        match flag.as_str() {
            "--json" => parsed.json = true,
            "--anonymous" => parsed.anonymous = true,
            "--loopback" => parsed.loopback = true,
            other => return Err(CliError::new(format!("unknown net-check argument {other}"))),
        }
    }
    Ok(parsed)
}

fn parse_host(args: &[String]) -> Result<HostArgs, CliError> {
    let mut draft = HostDraft {
        bind: "127.0.0.1:22116"
            .parse()
            .map_err(|_| CliError::new("default bind address is invalid"))?,
        approve: Vec::new(),
        allow_high_risk: false,
        screen: None,
        frames: 0,
        lan: false,
        relays: Vec::new(),
        files_root: None,
        tone: false,
        mic: false,
        speaker: false,
    };
    let mut index = 0;
    while index < args.len() {
        apply_host(&mut draft, args, &mut index)?;
        index += 1;
    }
    finish_host(draft)
}

fn apply_host(draft: &mut HostDraft, args: &[String], index: &mut usize) -> Result<(), CliError> {
    let flag = args[*index].as_str();
    match flag {
        "--bind" => {
            draft.bind = take(args, index, "bind address")?
                .parse()
                .map_err(|_| CliError::new("bind address is invalid"))?;
        }
        "--approve" => draft.approve = permissions(&take(args, index, "permissions")?)?,
        "--allow-high-risk" => draft.allow_high_risk = true,
        "--synthetic" => draft.screen = Some(false),
        "--screen" => draft.screen = Some(true),
        "--frames" => draft.frames = frames_arg(&take(args, index, "frame count")?)?,
        "--lan" => draft.lan = true,
        "--relay" => draft
            .relays
            .push(relay_arg(&take(args, index, "relay URL")?)?),
        "--files-root" => {
            draft.files_root = Some(PathBuf::from(take(args, index, "files root")?));
        }
        "--tone" => draft.tone = true,
        "--mic" => draft.mic = true,
        "--speaker" => draft.speaker = true,
        _ => return Err(CliError::new(format!("unknown host argument {flag}"))),
    }
    Ok(())
}

fn finish_host(draft: HostDraft) -> Result<HostArgs, CliError> {
    let screen = draft
        .screen
        .ok_or_else(|| CliError::new("host needs --synthetic or --screen"))?;
    if draft.approve.iter().any(|perm| perm.is_high_risk()) && !draft.allow_high_risk {
        return Err(CliError::new(
            "high-risk permissions need --allow-high-risk on first contact",
        ));
    }
    if needs_files_root(&draft.approve) && draft.files_root.is_none() {
        return Err(CliError::new("file_read and file_write need --files-root"));
    }
    if draft.tone && draft.mic {
        return Err(CliError::new("host takes --tone or --mic, not both"));
    }
    if (draft.tone || draft.mic || draft.speaker) && !draft.approve.contains(&Permission::Audio) {
        return Err(CliError::new(
            "microphone, speaker, and tone need audio in --approve",
        ));
    }
    Ok(HostArgs {
        bind: draft.bind,
        approve: draft.approve,
        allow_high_risk: draft.allow_high_risk,
        screen,
        frames: draft.frames,
        lan: draft.lan,
        relays: draft.relays,
        files_root: draft.files_root,
        tone: draft.tone,
        mic: draft.mic,
        speaker: draft.speaker,
    })
}

struct ViewDraft {
    ticket_hex: Option<String>,
    browse: bool,
    ppm: Option<PathBuf>,
    window: bool,
    pointer: Option<(i32, i32)>,
    key: Option<String>,
    put: Option<PathBuf>,
    audio: bool,
    mic: bool,
    speaker: bool,
    relays: Vec<String>,
}

fn parse_view(args: &[String]) -> Result<ViewArgs, CliError> {
    let mut draft = ViewDraft {
        ticket_hex: None,
        browse: false,
        ppm: None,
        window: false,
        pointer: None,
        key: None,
        put: None,
        audio: false,
        mic: false,
        speaker: false,
        relays: Vec::new(),
    };
    let mut index = 0;
    while index < args.len() {
        apply_view(&mut draft, args, &mut index)?;
        index += 1;
    }
    finish_view(draft)
}

fn apply_view(draft: &mut ViewDraft, args: &[String], index: &mut usize) -> Result<(), CliError> {
    let flag = args[*index].as_str();
    match flag {
        "--ticket" => draft.ticket_hex = Some(take(args, index, "ticket")?),
        "--browse" => draft.browse = true,
        "--ppm" => draft.ppm = Some(PathBuf::from(take(args, index, "ppm path")?)),
        "--window" => draft.window = true,
        "--pointer" => draft.pointer = Some(pointer_arg(&take(args, index, "pointer")?)?),
        "--key" => draft.key = Some(key_arg(&take(args, index, "key")?)?),
        "--put" => draft.put = Some(PathBuf::from(take(args, index, "file")?)),
        "--audio" => draft.audio = true,
        "--mic" => draft.mic = true,
        "--speaker" => draft.speaker = true,
        "--relay" => draft
            .relays
            .push(relay_arg(&take(args, index, "relay URL")?)?),
        _ => return Err(CliError::new(format!("unknown view argument {flag}"))),
    }
    Ok(())
}

fn finish_view(draft: ViewDraft) -> Result<ViewArgs, CliError> {
    if draft.ticket_hex.is_some() == draft.browse {
        return Err(CliError::new(
            "view needs exactly one of --ticket or --browse",
        ));
    }
    if draft.ppm.is_none()
        && !draft.window
        && draft.put.is_none()
        && !draft.audio
        && !draft.mic
        && !draft.speaker
    {
        return Err(CliError::new(
            "view needs --ppm, --window, --put, --audio, --mic, or --speaker",
        ));
    }
    Ok(ViewArgs {
        ticket_hex: draft.ticket_hex,
        browse: draft.browse,
        ppm: draft.ppm,
        window: draft.window,
        pointer: draft.pointer,
        key: draft.key,
        put: draft.put,
        audio: draft.audio,
        mic: draft.mic,
        speaker: draft.speaker,
        relays: draft.relays,
    })
}

fn needs_files_root(perms: &[Permission]) -> bool {
    perms
        .iter()
        .any(|perm| matches!(perm, Permission::FileRead | Permission::FileWrite))
}

fn take(args: &[String], index: &mut usize, name: &str) -> Result<String, CliError> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| CliError::new(format!("missing {name}")))
}

fn permissions(text: &str) -> Result<Vec<Permission>, CliError> {
    if text.is_empty() {
        return Err(CliError::new("approve list is empty"));
    }
    text.split(',').map(permission).collect()
}

fn permission(name: &str) -> Result<Permission, CliError> {
    let perm = match name {
        "view" => Permission::View,
        "input" => Permission::Input,
        "clipboard" => Permission::Clipboard,
        "file_read" => Permission::FileRead,
        "file_write" => Permission::FileWrite,
        "audio" => Permission::Audio,
        "terminal" => Permission::Terminal,
        "block_input" => Permission::BlockInput,
        "privacy_mode" => Permission::PrivacyMode,
        "sysinfo" => Permission::Sysinfo,
        _ => return Err(CliError::new(format!("unknown permission {name}"))),
    };
    Ok(perm)
}

fn frames_arg(text: &str) -> Result<u32, CliError> {
    let value: u32 = text
        .parse()
        .map_err(|_| CliError::new("frame count is invalid"))?;
    if value <= 30 {
        Ok(value)
    } else {
        Err(CliError::new(
            "frame count must be from 0 to 30; 0 streams until the viewer closes",
        ))
    }
}

fn pointer_arg(text: &str) -> Result<(i32, i32), CliError> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| CliError::new("pointer must be X,Y"))?;
    let x = x
        .parse()
        .map_err(|_| CliError::new("pointer x is invalid"))?;
    let y = y
        .parse()
        .map_err(|_| CliError::new("pointer y is invalid"))?;
    Ok((x, y))
}

fn key_arg(text: &str) -> Result<String, CliError> {
    crate::window::scripted_key(text)?;
    Ok(text.to_string())
}

fn relay_arg(text: &str) -> Result<String, CliError> {
    if text.starts_with("https://") || text.starts_with("relay://") {
        Ok(text.to_string())
    } else {
        Err(CliError::new("relay URL needs an https or relay scheme"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|text| (*text).to_string()).collect()
    }

    #[test]
    fn file_grant_needs_a_root_and_the_high_risk_flag() {
        let missing_flag = parse(&strings(&[
            "host",
            "--synthetic",
            "--approve",
            "file_write",
            "--files-root",
            "/tmp/cbdesk-files",
        ]));
        assert!(missing_flag.is_err());
        let missing_root = parse(&strings(&[
            "host",
            "--synthetic",
            "--approve",
            "file_read",
            "--allow-high-risk",
        ]));
        assert!(missing_root
            .expect_err("root")
            .to_string()
            .contains("files-root"));
    }

    #[test]
    fn tone_and_microphone_are_not_combined() {
        let both = parse(&strings(&[
            "host",
            "--synthetic",
            "--approve",
            "audio",
            "--tone",
            "--mic",
        ]));
        assert!(both.expect_err("both").to_string().contains("not both"));
        let missing = parse(&strings(&["host", "--synthetic", "--tone"]));
        assert!(missing.expect_err("audio").to_string().contains("audio"));
    }

    #[test]
    fn put_counts_as_a_view_action() {
        let command = parse(&strings(&[
            "view",
            "--ticket",
            "aa",
            "--put",
            "note.txt",
            "--relay",
            "https://203.0.113.9:443",
        ]))
        .expect("parse");
        match command {
            Command::View(view) => {
                assert!(view.ppm.is_none());
                assert!(view.put.is_some());
                assert_eq!(view.relays, vec!["https://203.0.113.9:443".to_string()]);
            }
            Command::Host(_) => panic!("host"),
            Command::NetCheck(_) => panic!("net-check"),
        }
    }

    #[test]
    fn net_check_and_doctor_share_flags() {
        let command =
            parse(&strings(&["doctor", "--json", "--anonymous", "--loopback"])).expect("parse");
        match command {
            Command::NetCheck(args) => {
                assert!(args.json && args.anonymous && args.loopback);
            }
            _ => panic!("net-check"),
        }
    }
}

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
}

/// Parse `args`, which does not include the program name.
pub fn parse(args: &[String]) -> Result<Command, CliError> {
    let Some(command) = args.first() else {
        return Err(usage());
    };
    match command.as_str() {
        "host" => Ok(Command::Host(parse_host(&args[1..])?)),
        "view" => Ok(Command::View(parse_view(&args[1..])?)),
        _ => Err(usage()),
    }
}

fn usage() -> CliError {
    CliError::new(
        "usage: cbdesk host --bind ADDR (--synthetic|--screen) [--approve view,input] [--frames N] [--lan]\n       cbdesk view (--ticket HEX|--browse) (--ppm FILE|--window) [--pointer X,Y]",
    )
}

fn parse_host(args: &[String]) -> Result<HostArgs, CliError> {
    let mut bind = "127.0.0.1:22116"
        .parse()
        .map_err(|_| CliError::new("default bind address is invalid"))?;
    let mut approve = Vec::new();
    let mut allow_high_risk = false;
    let mut screen = None;
    let mut frames = 0_u32;
    let mut lan = false;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--bind" => {
                bind = take(args, &mut index, "bind address")?
                    .parse()
                    .map_err(|_| CliError::new("bind address is invalid"))?
            }
            "--approve" => approve = permissions(&take(args, &mut index, "permissions")?)?,
            "--allow-high-risk" => allow_high_risk = true,
            "--synthetic" => screen = Some(false),
            "--screen" => screen = Some(true),
            "--frames" => frames = frames_arg(&take(args, &mut index, "frame count")?)?,
            "--lan" => lan = true,
            _ => return Err(CliError::new(format!("unknown host argument {flag}"))),
        }
        index += 1;
    }
    let screen = screen.ok_or_else(|| CliError::new("host needs --synthetic or --screen"))?;
    if approve.iter().any(|perm| perm.is_high_risk()) && !allow_high_risk {
        return Err(CliError::new(
            "high-risk permissions need --allow-high-risk on first contact",
        ));
    }
    Ok(HostArgs {
        bind,
        approve,
        allow_high_risk,
        screen,
        frames,
        lan,
    })
}

fn parse_view(args: &[String]) -> Result<ViewArgs, CliError> {
    let mut ticket_hex = None;
    let mut browse = false;
    let mut ppm = None;
    let mut window = false;
    let mut pointer = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--ticket" => ticket_hex = Some(take(args, &mut index, "ticket")?),
            "--browse" => browse = true,
            "--ppm" => ppm = Some(PathBuf::from(take(args, &mut index, "ppm path")?)),
            "--window" => window = true,
            "--pointer" => pointer = Some(pointer_arg(&take(args, &mut index, "pointer")?)?),
            _ => return Err(CliError::new(format!("unknown view argument {flag}"))),
        }
        index += 1;
    }
    if ticket_hex.is_some() == browse {
        return Err(CliError::new(
            "view needs exactly one of --ticket or --browse",
        ));
    }
    if ppm.is_none() && !window {
        return Err(CliError::new("view needs --ppm or --window"));
    }
    Ok(ViewArgs {
        ticket_hex,
        browse,
        ppm,
        window,
        pointer,
    })
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

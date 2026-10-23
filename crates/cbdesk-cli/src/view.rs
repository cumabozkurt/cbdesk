// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dial a host and show its frames.
//!
//! `--ppm` writes one picture and leaves. `--window` keeps reading until the
//! window closes, and forwards the pointer and keyboard only after an Input
//! grant. `--browse` finds the host on the LAN. A ticket is still only a
//! locator.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cbdesk_capture::Frame;
use cbdesk_codec::{decode_luma, decode_vp8_luma};
use cbdesk_crypto::Identity;
use cbdesk_discovery::ReplayCache;
use cbdesk_net::{
    dial_verified_ticket, open_controller, recv_clipboard_text, recv_view_frame,
    send_clipboard_text, send_input_event, ControllerGrant, ControllerSession, EndpointConfig,
    QuicConn, QuicEndpoint,
};
use cbdesk_proto::{
    local_capabilities, select_video_codec, AuthMethod, ClipboardText, Codec, Hello, InputBody,
    InputEvent, Permission, Platform, PointerEvent, ProtoMessage, UnknownFields, PROTO_MAJOR,
    PROTO_MINOR,
};
use zeroize::Zeroize;

use crate::args::ViewArgs;
use crate::clipboard::{ClipSync, OsClipboard};
use crate::error::CliError;
use crate::hex;
use crate::identity::open_identity;
use crate::lan::browse_peer;
use crate::ppm::write_ppm;
use crate::window::{self, WindowOptions};

struct LiveView {
    /// Dropping the endpoint closes the connection, so it stays here.
    _endpoint: QuicEndpoint,
    conn: QuicConn,
    grant: ControllerGrant,
    codec: Codec,
    _identity: Identity,
}

/// Dial and write one frame to a PPM file.
pub async fn view_remote(args: &ViewArgs) -> Result<(), CliError> {
    let live = connect_view(args).await?;
    let frame = read_frame(&live).await?;
    let Some(path) = args.ppm.as_ref() else {
        return Err(CliError::new("view needs --ppm or --window"));
    };
    write_ppm(path, &frame)?;
    send_scripted_pointer(&live, args.pointer).await?;
    let _ = live.grant.release().await;
    Ok(())
}

/// Dial, then run the window on this thread until it closes.
pub fn view_window(args: &ViewArgs) -> Result<(), CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| CliError::new(err.to_string()))?;
    let live = runtime.block_on(connect_view(args))?;
    let allow_input = live.grant.granted.perms.contains(&Permission::Input);
    let allow_clip = live.grant.granted.perms.contains(&Permission::Clipboard);
    let frames = Arc::new(Mutex::new(None));
    let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let ppm = args.ppm.clone();
    let pointer = args.pointer;
    let shown = Arc::clone(&frames);
    let task = runtime.spawn(async move {
        drive_window(WindowDrive {
            live,
            frames: shown,
            input_rx,
            stop_rx,
            allow_input,
            allow_clip,
            ppm,
            pointer,
        })
        .await
    });
    let window_result = window::run(
        frames,
        input_tx,
        stop_tx,
        WindowOptions {
            allow_input,
            any_thread: false,
            close_after: None,
        },
    );
    let net_result = runtime
        .block_on(task)
        .map_err(|err| CliError::new(err.to_string()))?;
    window_result?;
    net_result
}

struct WindowDrive {
    live: LiveView,
    frames: Arc<Mutex<Option<Frame>>>,
    input_rx: tokio::sync::mpsc::UnboundedReceiver<InputEvent>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    allow_input: bool,
    allow_clip: bool,
    ppm: Option<PathBuf>,
    pointer: Option<(i32, i32)>,
}

async fn drive_window(mut drive: WindowDrive) -> Result<(), CliError> {
    let mut clips = ClipSync::new();
    let mut wrote_ppm = false;
    let mut sent_pointer = false;
    loop {
        let pause = tokio::time::sleep(Duration::from_millis(400));
        tokio::pin!(pause);
        tokio::select! {
            biased;
            changed = drive.stop_rx.changed() => {
                if changed.is_err() || *drive.stop_rx.borrow() {
                    break;
                }
            }
            input = drive.input_rx.recv(), if drive.allow_input => {
                match input {
                    Some(event) => {
                        send_input_event(&drive.live.conn, &event).await.map_err(cli)?;
                    }
                    None => break,
                }
            }
            frame = recv_view_frame(&drive.live.conn) => {
                let picture = take_picture(drive.live.codec, frame.map_err(cli)?)?;
                if let Some(path) = drive.ppm.as_ref() {
                    if !wrote_ppm {
                        write_ppm(path, &picture)?;
                        wrote_ppm = true;
                    }
                }
                if let Ok(mut slot) = drive.frames.lock() {
                    *slot = Some(picture);
                }
                if !sent_pointer {
                    send_scripted_pointer(&drive.live, drive.pointer).await?;
                    sent_pointer = true;
                }
            }
            clip = recv_clipboard_text(&drive.live.conn), if drive.allow_clip => {
                apply_clip(&mut clips, clip.map_err(cli)?)?;
            }
            _ = &mut pause, if drive.allow_clip => {
                push_local_clip(&drive.live.conn, &mut clips).await?;
            }
        }
    }
    let _ = drive.live.grant.release().await;
    Ok(())
}

async fn connect_view(args: &ViewArgs) -> Result<LiveView, CliError> {
    let (endpoint, identity) = bind_viewer().await?;
    let conn = if args.browse {
        let (peer, addr) = browse_peer(&identity).await?;
        endpoint.connect(peer.as_bytes(), addr).await.map_err(cli)?
    } else {
        dial_ticket(&endpoint, args).await?
    };
    let grant = match open_controller(&conn, &hello(args)).await.map_err(cli)? {
        ControllerSession::Granted(grant) => grant,
        ControllerSession::Closed(close) => {
            return Err(CliError::new(format!("host closed: {:?}", close.code)));
        }
    };
    if !grant.granted.perms.contains(&Permission::View) {
        return Err(CliError::new("host did not grant view"));
    }
    let codec = select_video_codec(&local_capabilities(), &grant.host_caps)
        .ok_or_else(|| CliError::new("no shared video codec"))?;
    Ok(LiveView {
        _endpoint: endpoint,
        conn,
        grant,
        codec,
        _identity: identity,
    })
}

async fn dial_ticket(endpoint: &QuicEndpoint, args: &ViewArgs) -> Result<QuicConn, CliError> {
    let ticket_hex = args
        .ticket_hex
        .as_deref()
        .ok_or_else(|| CliError::new("view needs --ticket or --browse"))?;
    let bytes = hex::decode(ticket_hex.trim())?;
    let ticket = cbdesk_proto::InviteTicket::from_bytes(&bytes).map_err(cli)?;
    let mut replay = ReplayCache::new();
    dial_verified_ticket(endpoint, &ticket, unix_now(), &mut replay)
        .await
        .map_err(cli)
}

async fn read_frame(live: &LiveView) -> Result<Frame, CliError> {
    take_picture(live.codec, recv_view_frame(&live.conn).await.map_err(cli)?)
}

fn take_picture(
    expected: Codec,
    frame: (cbdesk_proto::VideoFrameHeader, Vec<u8>),
) -> Result<Frame, CliError> {
    let (header, payload) = frame;
    if header.codec != expected {
        return Err(CliError::new("host sent a codec that was not negotiated"));
    }
    decode_picture(header.codec, &payload)
}

fn decode_picture(codec: Codec, payload: &[u8]) -> Result<Frame, CliError> {
    let frame = match codec {
        Codec::Vp8 => decode_vp8_luma(payload),
        Codec::Av1 => decode_luma(payload),
        _ => return Err(CliError::new("host sent a codec this build cannot decode")),
    };
    frame.map_err(cli)
}

async fn send_scripted_pointer(
    live: &LiveView,
    pointer: Option<(i32, i32)>,
) -> Result<(), CliError> {
    let Some((x, y)) = pointer else {
        return Ok(());
    };
    if !live.grant.granted.perms.contains(&Permission::Input) {
        return Err(CliError::new("host did not grant input"));
    }
    let event = InputEvent {
        body: InputBody::Pointer(PointerEvent {
            x,
            y,
            buttons: 0,
            wheel: 0,
            unknown: UnknownFields::new(),
        }),
        unknown: UnknownFields::new(),
    };
    send_input_event(&live.conn, &event).await.map_err(cli)
}

fn apply_clip(clips: &mut ClipSync, text: ClipboardText) -> Result<(), CliError> {
    if !clips.note_remote(&text.text) || clips.os_broken() {
        return Ok(());
    }
    if let Err(err) = OsClipboard::set(&text.text) {
        clips.mark_os_broken();
        eprintln!("cbdesk: clipboard is unavailable: {err}");
    }
    Ok(())
}

async fn push_local_clip(conn: &QuicConn, clips: &mut ClipSync) -> Result<(), CliError> {
    if clips.os_broken() {
        return Ok(());
    }
    let text = match OsClipboard::get() {
        Ok(Some(text)) => text,
        Ok(None) => return Ok(()),
        Err(err) => {
            clips.mark_os_broken();
            eprintln!("cbdesk: clipboard is unavailable: {err}");
            return Ok(());
        }
    };
    let Some(generation) = clips.local_change(&text) else {
        return Ok(());
    };
    let message = ClipboardText {
        text,
        generation,
        unknown: UnknownFields::new(),
    };
    send_clipboard_text(conn, &message).await.map_err(cli)
}

async fn bind_viewer() -> Result<(QuicEndpoint, Identity), CliError> {
    let identity = open_identity()?;
    let mut seed = identity.signing_seed();
    let addr = "127.0.0.1:0"
        .parse()
        .map_err(|_| CliError::new("viewer bind address is invalid"))?;
    let endpoint = QuicEndpoint::bind(&EndpointConfig::default(), addr, &seed)
        .await
        .map_err(cli)?;
    seed.zeroize();
    Ok((endpoint, identity))
}

fn hello(args: &ViewArgs) -> Hello {
    let mut wanted = vec![Permission::View];
    if args.pointer.is_some() || args.window {
        wanted.push(Permission::Input);
    }
    if args.window {
        wanted.push(Permission::Clipboard);
    }
    Hello {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        app_version: "0.0.0".to_string(),
        platform: local_platform(),
        caps: local_capabilities().to_vec(),
        wanted_perms: wanted,
        auth_offer: vec![AuthMethod::Interactive],
        unknown: UnknownFields::new(),
        ..Hello::default()
    }
}

fn local_platform() -> Platform {
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        Platform::Macos
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Platform::Unspecified
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn cli(err: impl ToString) -> CliError {
    CliError::new(err.to_string())
}

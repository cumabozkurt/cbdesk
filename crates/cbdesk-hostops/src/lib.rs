// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Operating-system effects behind a session permit.
//!
//! Privacy mode and block-input on Linux talk to the X server named by the
//! caller. A virtual display is `Xvfb` from `cbdesk-capture`, not a Windows
//! indirect display driver. The terminal is a pseudoterminal on Unix and
//! `cmd.exe` pipes on Windows. The tunnel listens on `127.0.0.1` and refuses
//! any other destination. Recording writes a notice file and may call
//! `notify-send`. None of these functions turn the session indicator off.

mod error;
mod notice;
mod terminal;
mod tunnel;

#[cfg(target_os = "linux")]
mod x11;

pub use error::OsError;
pub use notice::{post_recording_notice, NoticeReport};
pub use terminal::HostShell;
pub use tunnel::LoopbackTunnel;

#[cfg(target_os = "linux")]
pub use x11::{block_local_input, cover_display, InputGrab, PrivacyCover};

/// Grab local input. Non-Linux builds have no grab.
#[cfg(not(target_os = "linux"))]
pub fn block_local_input(
    permit: cbdesk_session::OsPermit,
    _display: Option<&str>,
) -> Result<Unavailable, OsError> {
    let _ = permit.kind();
    Err(OsError::Unavailable(
        "block-input is implemented for an X11 display; this platform has no grab".to_string(),
    ))
}

/// Cover the local display. Non-Linux builds have no cover.
#[cfg(not(target_os = "linux"))]
pub fn cover_display(
    permit: cbdesk_session::OsPermit,
    _display: Option<&str>,
) -> Result<Unavailable, OsError> {
    let _ = permit.kind();
    Err(OsError::Unavailable(
        "privacy mode is implemented as an X11 window; this platform has no cover".to_string(),
    ))
}

/// Placeholder so non-Linux signatures still name a type.
#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub struct Unavailable;

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    use cbdesk_proto::Permission;
    use cbdesk_session::{
        open_terminal, open_tunnel, request_block_input, request_privacy_mode, request_recording,
        PermissionSet,
    };

    use super::*;

    fn granted(permission: Permission) -> PermissionSet {
        let mut set = PermissionSet::empty();
        set.insert(permission);
        set
    }

    #[test]
    fn a_shell_prints_a_marker_and_a_tunnel_stays_on_loopback() {
        let mut shell =
            HostShell::spawn(open_terminal(granted(Permission::Terminal), true).expect("permit"))
                .expect("shell");
        let command: &[u8] = if cfg!(windows) {
            b"echo cbdesk-terminal\r\n"
        } else {
            b"printf cbdesk-terminal\n"
        };
        shell.write_all(command).expect("write");
        let output = shell
            .read_until(b"cbdesk-terminal", Duration::from_secs(5))
            .expect("read");
        assert!(output
            .windows(b"cbdesk-terminal".len())
            .any(|window| window == b"cbdesk-terminal"));
        assert!(
            HostShell::spawn(open_tunnel(granted(Permission::Tunnel), true).expect("tunnel"))
                .is_err()
        );

        let dest = TcpListener::bind("127.0.0.1:0").expect("dest");
        let dest_addr = dest.local_addr().expect("addr");
        let tunnel =
            LoopbackTunnel::bind(open_tunnel(granted(Permission::Tunnel), true).expect("permit"))
                .expect("bind");
        let tunnel_addr = tunnel.local_addr().expect("tunnel addr");
        assert!(tunnel
            .forward_one("8.8.8.8:9".parse().expect("ip"))
            .is_err());
        let copied = thread::spawn(move || tunnel.forward_one(dest_addr));
        let mut client = std::net::TcpStream::connect(tunnel_addr).expect("client");
        client.write_all(b"ping").expect("ping");
        let (mut inbound, _) = dest.accept().expect("accept");
        let mut buf = [0_u8; 4];
        inbound.read_exact(&mut buf).expect("exact");
        assert_eq!(&buf, b"ping");
        copied.join().expect("join").expect("forward");
    }

    #[test]
    fn recording_notice_is_a_file_before_it_is_a_desktop_popup() {
        let path = std::env::temp_dir().join(format!("cbdesk-notice-{}", std::process::id()));
        let report = post_recording_notice(
            request_recording(granted(Permission::Record), true, true).expect("permit"),
            "the host was told",
            &path,
        )
        .expect("notice");
        let text = std::fs::read_to_string(&report.path).expect("read");
        assert!(text.contains("CBDesk recording"));
        assert!(text.contains("the host was told"));
        assert!(post_recording_notice(
            request_privacy_mode(granted(Permission::PrivacyMode), true).expect("privacy"),
            "no",
            &path,
        )
        .is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn x11_effects_run_on_a_private_server_when_one_can_start() {
        let display = match cbdesk_capture::open_virtual_display() {
            Ok(display) => display,
            Err(err) => {
                let required = cfg!(target_os = "linux")
                    && std::env::var_os("CBDESK_REQUIRE_DISPLAY").is_some();
                assert!(!required, "Xvfb was required: {err}");
                return;
            }
        };
        let name = display.name().to_string();
        let first = block_local_input(
            request_block_input(granted(Permission::BlockInput), true).expect("permit"),
            Some(&name),
        )
        .expect("grab");
        drop(first);
        let second = block_local_input(
            request_block_input(granted(Permission::BlockInput), true).expect("permit"),
            Some(&name),
        )
        .expect("second grab");
        drop(second);
        let cover = cover_display(
            request_privacy_mode(granted(Permission::PrivacyMode), true).expect("permit"),
            Some(&name),
        )
        .expect("cover");
        let (width, height) = cover.mapped_size().expect("size");
        assert_eq!((width, height), (1280, 720));
    }
}

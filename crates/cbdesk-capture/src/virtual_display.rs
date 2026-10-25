// SPDX-License-Identifier: AGPL-3.0-or-later

//! A userspace virtual display.
//!
//! On Linux this starts `Xvfb` when that binary is on `PATH` and stops it
//! when the value is dropped. It is an X server with no hardware, not a
//! Windows indirect display driver. That driver is not in this tree, and its
//! signing process has not started. macOS has no virtual-display entitlement
//! here. A missing `Xvfb` is an error.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::CaptureError;

static NEXT_DISPLAY: AtomicU32 = AtomicU32::new(0);

/// An Xvfb process, or the reason another platform has none.
pub struct VirtualDisplay {
    name: String,
    child: Child,
}

impl VirtualDisplay {
    /// The X11 display name, including the leading colon.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for VirtualDisplay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start a virtual display, or explain why this platform cannot.
pub fn open_virtual_display() -> Result<VirtualDisplay, CaptureError> {
    if !cfg!(target_os = "linux") {
        return Err(CaptureError::Unavailable(
            "virtual display on this platform needs a signed driver that is not in this tree"
                .to_string(),
        ));
    }
    if which("Xvfb").is_none() {
        return Err(CaptureError::Unavailable(
            "Xvfb is not installed".to_string(),
        ));
    }
    let mut last = String::from("Xvfb did not stay up");
    for _ in 0..20 {
        let number = display_number();
        let name = format!(":{number}");
        let socket = format!("/tmp/.X11-unix/X{number}");
        if Path::new(&socket).exists() {
            continue;
        }
        match spawn_xvfb(&name) {
            Ok(child) => {
                if wait_for_socket(&socket, &name) {
                    return Ok(VirtualDisplay { name, child });
                }
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                last = format!("Xvfb {name} exited before the socket appeared");
            }
            Err(err) => last = err,
        }
    }
    Err(CaptureError::Unavailable(last))
}

fn display_number() -> u32 {
    let step = NEXT_DISPLAY.fetch_add(1, Ordering::Relaxed);
    120 + (std::process::id() % 4_000) * 8 + step
}

fn spawn_xvfb(name: &str) -> Result<Child, String> {
    Command::new("Xvfb")
        .args([
            name,
            "-screen",
            "0",
            "1280x720x24",
            "-nolisten",
            "tcp",
            "-ac",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| err.to_string())
}

fn wait_for_socket(socket: &str, name: &str) -> bool {
    // The socket file appears before Xvfb finishes its first handshake.
    // `xdpyinfo` is the same readiness check CI uses for `DISPLAY=:99`.
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if display_answers(name) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    Path::new(socket).exists() && display_answers(name)
}

fn display_answers(name: &str) -> bool {
    Command::new("xdpyinfo")
        .env("DISPLAY", name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_virtual_display_is_real_only_where_xvfb_can_start() {
        match open_virtual_display() {
            Ok(display) => {
                assert!(display.name().starts_with(':'));
                assert!(Path::new(&format!("/tmp/.X11-unix/X{}", &display.name()[1..])).exists());
            }
            Err(err) => {
                let required = cfg!(target_os = "linux")
                    && std::env::var_os("CBDESK_REQUIRE_DISPLAY").is_some();
                assert!(!required, "Xvfb was required: {err}");
            }
        }
    }
}

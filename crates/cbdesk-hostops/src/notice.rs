// SPDX-License-Identifier: AGPL-3.0-or-later

//! A recording notice the person at the host can see.
//!
//! The notice is a file. On Unix, `notify-send` is also tried when it is on
//! `PATH`. A missing notification daemon leaves [`NoticeReport::desktop`]
//! false. This function does not open a video file.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cbdesk_session::{OsPermit, PermitKind};

use crate::OsError;

/// Where the recording notice went.
#[derive(Debug)]
pub struct NoticeReport {
    /// True only when a desktop notification process exited 0.
    pub desktop: bool,
    /// The file that now contains the notice.
    pub path: PathBuf,
}

/// Write `text` and try a desktop notification.
pub fn post_recording_notice(
    permit: OsPermit,
    text: &str,
    path: &Path,
) -> Result<NoticeReport, OsError> {
    if permit.kind() != PermitKind::Recording {
        return Err(OsError::WrongPermit);
    }
    let body = format!("CBDesk recording\n{text}\n");
    std::fs::write(path, body).map_err(|err| OsError::Failed(err.to_string()))?;
    Ok(NoticeReport {
        desktop: desktop_notify(text),
        path: path.to_path_buf(),
    })
}

fn desktop_notify(text: &str) -> bool {
    if !cfg!(unix) {
        return false;
    }
    Command::new("notify-send")
        .args(["CBDesk recording", text])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

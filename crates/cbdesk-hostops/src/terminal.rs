// SPDX-License-Identifier: AGPL-3.0-or-later

//! A consented shell.
//!
//! Unix opens a pseudoterminal and starts `sh` with the slave as its standard
//! streams. Windows starts `cmd.exe` with pipes, because this crate does not
//! link ConPTY. Dropping the value kills the child. The permit is checked
//! before any process is created.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use cbdesk_session::{OsPermit, PermitKind};

use crate::OsError;

/// A shell started for a terminal permit.
pub struct HostShell {
    child: Child,
    input: Writer,
    output: Option<Reader>,
}

impl HostShell {
    /// Start a shell. Nothing is spawned when `permit` is not a terminal permit.
    pub fn spawn(permit: OsPermit) -> Result<Self, OsError> {
        if permit.kind() != PermitKind::Terminal {
            return Err(OsError::WrongPermit);
        }
        spawn_shell()
    }

    /// Write bytes to the shell.
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), OsError> {
        self.input
            .write_all(bytes)
            .map_err(|err| OsError::Failed(err.to_string()))?;
        self.input
            .flush()
            .map_err(|err| OsError::Failed(err.to_string()))
    }

    /// Read until `marker` appears or `timeout` elapses.
    pub fn read_until(&mut self, marker: &[u8], timeout: Duration) -> Result<Vec<u8>, OsError> {
        let output = self
            .output
            .take()
            .ok_or_else(|| OsError::Failed("terminal output is already borrowed".to_string()))?;
        let marker = marker.to_vec();
        let wanted = marker.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut output = output;
            let mut buf = Vec::new();
            let mut tmp = [0_u8; 256];
            loop {
                if !wanted.is_empty()
                    && buf.len() >= wanted.len()
                    && buf
                        .windows(wanted.len())
                        .any(|window| window == wanted.as_slice())
                {
                    break;
                }
                match output.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => buf.extend_from_slice(&tmp[..count]),
                }
            }
            let _ = tx.send((buf, output));
        });
        match rx.recv_timeout(timeout) {
            Ok((buf, output)) => {
                self.output = Some(output);
                if !marker.is_empty()
                    && buf
                        .windows(marker.len())
                        .any(|window| window == marker.as_slice())
                {
                    Ok(buf)
                } else {
                    Err(OsError::Failed(
                        "the shell exited before it printed the marker".to_string(),
                    ))
                }
            }
            Err(_) => {
                let _ = self.child.kill();
                if let Ok((_buf, output)) = rx.recv_timeout(Duration::from_secs(1)) {
                    self.output = Some(output);
                }
                Err(OsError::Failed("terminal read timed out".to_string()))
            }
        }
    }
}

impl Drop for HostShell {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

enum Writer {
    #[cfg(unix)]
    File(std::fs::File),
    #[cfg(windows)]
    Pipe(std::process::ChildStdin),
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::File(file) => file.write(buf),
            #[cfg(windows)]
            Self::Pipe(pipe) => pipe.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::File(file) => file.flush(),
            #[cfg(windows)]
            Self::Pipe(pipe) => pipe.flush(),
        }
    }
}

enum Reader {
    #[cfg(unix)]
    File(std::fs::File),
    #[cfg(windows)]
    Pipe(std::process::ChildStdout),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::File(file) => file.read(buf),
            #[cfg(windows)]
            Self::Pipe(pipe) => pipe.read(buf),
        }
    }
}

#[cfg(unix)]
fn spawn_shell() -> Result<HostShell, OsError> {
    use rustix::pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags};

    let mut flags = OpenptFlags::RDWR | OpenptFlags::NOCTTY;
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
    {
        flags |= OpenptFlags::CLOEXEC;
    }
    let master = openpt(flags).map_err(|err| OsError::Failed(err.to_string()))?;
    grantpt(&master).map_err(|err| OsError::Failed(err.to_string()))?;
    unlockpt(&master).map_err(|err| OsError::Failed(err.to_string()))?;
    let name = ptsname(&master, vec![0_u8; 128]).map_err(|err| OsError::Failed(err.to_string()))?;
    let slave = rustix::fs::open(
        name.as_c_str(),
        rustix::fs::OFlags::RDWR,
        rustix::fs::Mode::empty(),
    )
    .map_err(|err| OsError::Failed(err.to_string()))?;
    let slave_in = slave
        .try_clone()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    let slave_out = slave
        .try_clone()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    let child = Command::new("sh")
        .stdin(Stdio::from(slave_in))
        .stdout(Stdio::from(slave_out))
        .stderr(Stdio::from(slave))
        .spawn()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    let master = std::fs::File::from(master);
    let output = master
        .try_clone()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    Ok(HostShell {
        child,
        input: Writer::File(master),
        output: Some(Reader::File(output)),
    })
}

#[cfg(windows)]
fn spawn_shell() -> Result<HostShell, OsError> {
    let mut child = Command::new("cmd.exe")
        .arg("/Q")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| OsError::Failed("cmd stdin was not piped".to_string()))?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| OsError::Failed("cmd stdout was not piped".to_string()))?;
    Ok(HostShell {
        child,
        input: Writer::Pipe(input),
        output: Some(Reader::Pipe(output)),
    })
}

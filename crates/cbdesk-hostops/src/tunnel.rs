// SPDX-License-Identifier: AGPL-3.0-or-later

//! One loopback TCP forward.
//!
//! The listener is `127.0.0.1`. [`LoopbackTunnel::forward_one`] refuses any
//! destination that is not a loopback address, and it copies a single read.
//! It does not listen on `0.0.0.0`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use cbdesk_session::{OsPermit, PermitKind};

use crate::OsError;

/// A TCP listener bound to loopback.
pub struct LoopbackTunnel {
    listener: TcpListener,
}

impl LoopbackTunnel {
    /// Bind `127.0.0.1:0`.
    pub fn bind(permit: OsPermit) -> Result<Self, OsError> {
        if permit.kind() != PermitKind::Tunnel {
            return Err(OsError::WrongPermit);
        }
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|err| OsError::Failed(err.to_string()))?;
        Ok(Self { listener })
    }

    /// The address the listener accepted.
    pub fn local_addr(&self) -> Result<SocketAddr, OsError> {
        self.listener
            .local_addr()
            .map_err(|err| OsError::Failed(err.to_string()))
    }

    /// Accept one connection and copy one read to `dest`.
    ///
    /// `dest` must be a loopback address. The check happens before `accept`.
    pub fn forward_one(&self, dest: SocketAddr) -> Result<(), OsError> {
        if !dest.ip().is_loopback() {
            return Err(OsError::Refused(
                "destination is not a loopback address".to_string(),
            ));
        }
        let (mut client, _) = self
            .listener
            .accept()
            .map_err(|err| OsError::Failed(err.to_string()))?;
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|err| OsError::Failed(err.to_string()))?;
        let mut remote = TcpStream::connect_timeout(&dest, Duration::from_secs(2))
            .map_err(|err| OsError::Failed(err.to_string()))?;
        let mut buf = [0_u8; 4096];
        let count = client
            .read(&mut buf)
            .map_err(|err| OsError::Failed(err.to_string()))?;
        remote
            .write_all(&buf[..count])
            .map_err(|err| OsError::Failed(err.to_string()))?;
        Ok(())
    }
}

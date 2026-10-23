// SPDX-License-Identifier: AGPL-3.0-or-later

//! Length-prefixed control frames on one bidirectional QUIC stream.
//!
//! The length is a big-endian `u32` and counts only the protobuf [`Control`]
//! message. It must be between 1 and 64 KiB. Bytes that are not that frame,
//! including an echo probe, are [`NetError::BadControl`].

use cbdesk_proto::{Control, ProtoMessage, CONTROL_MAX_BYTES};

use crate::error::NetError;
use crate::quic::QuicConn;

/// The control stream for one connection.
pub(crate) struct ControlChannel {
    send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
}

impl std::fmt::Debug for ControlChannel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ControlChannel")
    }
}

impl ControlChannel {
    pub(crate) async fn open(conn: &QuicConn) -> Result<Self, NetError> {
        let (send, recv) = conn.open_bi_streams().await?;
        Ok(Self { send, recv })
    }

    pub(crate) async fn accept(conn: &QuicConn) -> Result<Self, NetError> {
        let (send, recv) = conn.accept_bi_streams().await?;
        Ok(Self { send, recv })
    }

    pub(crate) async fn write(&mut self, message: &Control) -> Result<(), NetError> {
        let body = message.to_bytes().map_err(|_| NetError::BadControl)?;
        let len = u32::try_from(body.len()).map_err(|_| NetError::BadControl)?;
        let mut frame = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&body);
        self.send
            .write_all(&frame)
            .await
            .map_err(|err| NetError::Quic(err.to_string()))
    }

    pub(crate) async fn read(&mut self) -> Result<Control, NetError> {
        let mut len_buf = [0_u8; 4];
        self.recv
            .read_exact(&mut len_buf)
            .await
            .map_err(|_| NetError::BadControl)?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 || len > CONTROL_MAX_BYTES {
            return Err(NetError::BadControl);
        }
        let mut body = vec![0_u8; len];
        self.recv
            .read_exact(&mut body)
            .await
            .map_err(|_| NetError::BadControl)?;
        Control::from_bytes(&body).map_err(|_| NetError::BadControl)
    }

    pub(crate) fn finish_send(&mut self) -> Result<(), NetError> {
        self.send
            .finish()
            .map_err(|err| NetError::Quic(err.to_string()))
    }
}

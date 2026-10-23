// SPDX-License-Identifier: AGPL-3.0-or-later

//! SPAKE2 on the first bidirectional stream.
//!
//! The magic is `CBPK`. It cannot be a control-frame length, because a
//! control frame is at most 64 KiB. A desk host that sees any other first
//! word closes the stream and does not ask for consent. The CLI host does
//! not call this module, so a controller that speaks control first still
//! reaches the CLI consent path.

use cbdesk_crypto::{tags_match, transcript, Handshake, Side};
use zeroize::Zeroize;

use crate::error::NetError;
use crate::quic::QuicConn;

/// First word of a pairing stream. ASCII `CBPK`.
pub const PAKE_MAGIC: u32 = 0x4342_504B;

const MAX_MESSAGE: usize = 256;

/// Words, slot, and this device's Ed25519 key. The words are wiped on drop.
pub struct Pairing {
    password: Vec<u8>,
    slot: u16,
    local_pk: [u8; 32],
    side: Side,
}

impl Pairing {
    /// Host side. `password` is the two lowercase words, not the slot.
    pub fn host(password: Vec<u8>, slot: u16, local_pk: [u8; 32]) -> Self {
        Self {
            password,
            slot,
            local_pk,
            side: Side::Host,
        }
    }

    /// Controller side. The remote key is read from the connection.
    pub fn controller(password: Vec<u8>, slot: u16, local_pk: [u8; 32]) -> Self {
        Self {
            password,
            slot,
            local_pk,
            side: Side::Controller,
        }
    }
}

impl Drop for Pairing {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

/// Controller opens the stream, then the host answers.
pub async fn offer_pairing(conn: &QuicConn, pairing: &Pairing) -> Result<(), NetError> {
    let remote = conn.remote_device_id();
    let (send, recv) = conn.open_bi_streams().await?;
    let result = controller_flow(send, recv, pairing, &remote).await;
    result
}

/// Host accepts the stream. Anything except [`PAKE_MAGIC`] is refused.
pub async fn accept_pairing(conn: &QuicConn, pairing: &Pairing) -> Result<(), NetError> {
    let remote = conn.remote_device_id();
    let (send, recv) = conn.accept_bi_streams().await?;
    host_flow(send, recv, pairing, &remote).await
}

async fn controller_flow(
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    pairing: &Pairing,
    remote_pk: &[u8; 32],
) -> Result<(), NetError> {
    let bound = transcript_for(pairing, remote_pk)?;
    let (handshake, message) = Handshake::start_controller(&pairing.password, &bound)
        .map_err(|_| NetError::PairingFailed)?;
    write_u32(&mut send, PAKE_MAGIC).await?;
    write_blob(&mut send, &message).await?;
    let peer = read_blob(&mut recv).await?;
    let tags = handshake
        .finish(&peer)
        .map_err(|_| NetError::PairingFailed)?;
    write_blob(&mut send, &tags.local_tag).await?;
    let peer_tag = read_blob(&mut recv).await?;
    finish_send(&mut send).await?;
    if !tags_match(&peer_tag, &tags.peer_tag) {
        return Err(NetError::PairingFailed);
    }
    Ok(())
}

async fn host_flow(
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    pairing: &Pairing,
    remote_pk: &[u8; 32],
) -> Result<(), NetError> {
    let magic = read_u32(&mut recv).await?;
    if magic != PAKE_MAGIC {
        return Err(NetError::PairingRequired);
    }
    let peer = read_blob(&mut recv).await?;
    let bound = transcript_for(pairing, remote_pk)?;
    let (handshake, message) =
        Handshake::start_host(&pairing.password, &bound).map_err(|_| NetError::PairingFailed)?;
    let tags = handshake
        .finish(&peer)
        .map_err(|_| NetError::PairingFailed)?;
    write_blob(&mut send, &message).await?;
    let peer_tag = read_blob(&mut recv).await?;
    if !tags_match(&peer_tag, &tags.peer_tag) {
        return Err(NetError::PairingFailed);
    }
    write_blob(&mut send, &tags.local_tag).await?;
    finish_send(&mut send).await?;
    Ok(())
}

fn transcript_for(pairing: &Pairing, remote_pk: &[u8; 32]) -> Result<Vec<u8>, NetError> {
    let (host_pk, controller_pk) = match pairing.side {
        Side::Host => (&pairing.local_pk, remote_pk),
        Side::Controller => (remote_pk, &pairing.local_pk),
    };
    Ok(transcript(pairing.slot, host_pk, controller_pk))
}

async fn write_u32(send: &mut iroh::endpoint::SendStream, value: u32) -> Result<(), NetError> {
    send.write_all(&value.to_be_bytes())
        .await
        .map_err(|err| NetError::Quic(err.to_string()))
}

async fn write_blob(send: &mut iroh::endpoint::SendStream, bytes: &[u8]) -> Result<(), NetError> {
    let len = u32::try_from(bytes.len()).map_err(|_| NetError::PairingFailed)?;
    write_u32(send, len).await?;
    send.write_all(bytes)
        .await
        .map_err(|err| NetError::Quic(err.to_string()))
}

async fn read_u32(recv: &mut iroh::endpoint::RecvStream) -> Result<u32, NetError> {
    let mut buf = [0_u8; 4];
    recv.read_exact(&mut buf)
        .await
        .map_err(|_| NetError::PairingFailed)?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_blob(recv: &mut iroh::endpoint::RecvStream) -> Result<Vec<u8>, NetError> {
    let len = read_u32(recv).await? as usize;
    if len == 0 || len > MAX_MESSAGE {
        return Err(NetError::PairingFailed);
    }
    let mut buf = vec![0_u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(|_| NetError::PairingFailed)?;
    Ok(buf)
}

async fn finish_send(send: &mut iroh::endpoint::SendStream) -> Result<(), NetError> {
    send.finish()
        .map_err(|err| NetError::Quic(err.to_string()))?;
    let _ = send.stopped().await;
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! One video frame or one input event on its own unidirectional stream, and
//! one clipboard text update on a bidirectional stream.
//!
//! A frame is a big-endian `u32` length, a `VideoFrameHeader`, a big-endian
//! `u32` length, and the payload. The lengths plus both bodies stay within
//! 16 MiB. An input event is a big-endian `u32` length and one `InputEvent`.
//! Clipboard text uses the same length prefix on a bidirectional stream.
//! Opening a stream is the caller's decision; this module only frames bytes.
//! See ADR-014.

use cbdesk_proto::{
    ClipboardText, InputEvent, ProtoMessage, VideoFrameHeader, CONTROL_MAX_BYTES,
    VIDEO_FRAME_MAX_BYTES,
};

use crate::error::NetError;
use crate::quic::QuicConn;

/// Read one video frame the host opened after `authorize(View)`.
pub async fn recv_view_frame(conn: &QuicConn) -> Result<(VideoFrameHeader, Vec<u8>), NetError> {
    let mut recv = conn.accept_uni_stream().await?;
    read_view_frame(&mut recv).await
}

/// Send one clipboard text update. The host still has to have granted Clipboard.
pub async fn send_clipboard_text(conn: &QuicConn, text: &ClipboardText) -> Result<(), NetError> {
    let (mut send, _recv) = conn.open_bi_streams().await?;
    write_clipboard(&mut send, text).await?;
    send.finish()
        .map_err(|err| NetError::Quic(err.to_string()))?;
    Ok(())
}

/// Read one clipboard text update the peer opened after `authorize(Clipboard)`.
pub async fn recv_clipboard_text(conn: &QuicConn) -> Result<ClipboardText, NetError> {
    let (mut send, mut recv) = conn.accept_bi_streams().await?;
    let text = read_clipboard(&mut recv).await?;
    let _ = send.finish();
    Ok(text)
}

/// Send one input event. The host still has to have granted Input.
pub async fn send_input_event(conn: &QuicConn, event: &InputEvent) -> Result<(), NetError> {
    let mut send = conn.open_uni_stream().await?;
    write_input(&mut send, event).await?;
    send.finish()
        .map_err(|err| NetError::Quic(err.to_string()))?;
    Ok(())
}

pub(crate) fn validate_view_frame(
    header: &VideoFrameHeader,
    payload: &[u8],
) -> Result<(), NetError> {
    let header_bytes = header.to_bytes().map_err(|_| NetError::BadMedia)?;
    check_frame(header_bytes.len(), payload.len())
}

pub(crate) async fn write_view_frame(
    send: &mut iroh::endpoint::SendStream,
    header: &VideoFrameHeader,
    payload: &[u8],
) -> Result<(), NetError> {
    let header_bytes = header.to_bytes().map_err(|_| NetError::BadMedia)?;
    check_frame(header_bytes.len(), payload.len())?;
    write_prefixed(send, &header_bytes, CONTROL_MAX_BYTES).await?;
    write_prefixed(send, payload, VIDEO_FRAME_MAX_BYTES).await?;
    send.finish()
        .map_err(|err| NetError::Quic(err.to_string()))?;
    Ok(())
}

pub(crate) async fn read_view_frame(
    recv: &mut iroh::endpoint::RecvStream,
) -> Result<(VideoFrameHeader, Vec<u8>), NetError> {
    let header_bytes = read_prefixed(recv, CONTROL_MAX_BYTES).await?;
    let header = VideoFrameHeader::from_bytes(&header_bytes).map_err(|_| NetError::BadMedia)?;
    let payload = read_prefixed(recv, VIDEO_FRAME_MAX_BYTES).await?;
    check_frame(header_bytes.len(), payload.len())?;
    Ok((header, payload))
}

pub(crate) fn check_frame(header_len: usize, payload_len: usize) -> Result<(), NetError> {
    if payload_len == 0 || header_len == 0 {
        return Err(NetError::BadMedia);
    }
    let total = 8usize
        .saturating_add(header_len)
        .saturating_add(payload_len);
    if total > VIDEO_FRAME_MAX_BYTES {
        return Err(NetError::BadMedia);
    }
    Ok(())
}

async fn write_input(
    send: &mut iroh::endpoint::SendStream,
    event: &InputEvent,
) -> Result<(), NetError> {
    let bytes = event.to_bytes().map_err(|_| NetError::BadMedia)?;
    write_prefixed(send, &bytes, CONTROL_MAX_BYTES).await
}

pub(crate) async fn read_input(
    recv: &mut iroh::endpoint::RecvStream,
) -> Result<InputEvent, NetError> {
    let bytes = read_prefixed(recv, CONTROL_MAX_BYTES).await?;
    InputEvent::from_bytes(&bytes).map_err(|_| NetError::BadMedia)
}

pub(crate) async fn write_clipboard(
    send: &mut iroh::endpoint::SendStream,
    text: &ClipboardText,
) -> Result<(), NetError> {
    let bytes = text.to_bytes().map_err(|_| NetError::BadMedia)?;
    write_prefixed(send, &bytes, CONTROL_MAX_BYTES).await
}

pub(crate) async fn read_clipboard(
    recv: &mut iroh::endpoint::RecvStream,
) -> Result<ClipboardText, NetError> {
    let bytes = read_prefixed(recv, CONTROL_MAX_BYTES).await?;
    ClipboardText::from_bytes(&bytes).map_err(|_| NetError::BadMedia)
}

async fn write_prefixed(
    send: &mut iroh::endpoint::SendStream,
    bytes: &[u8],
    limit: usize,
) -> Result<(), NetError> {
    if bytes.is_empty() || bytes.len() > limit {
        return Err(NetError::BadMedia);
    }
    let len = u32::try_from(bytes.len()).map_err(|_| NetError::BadMedia)?;
    send.write_all(&len.to_be_bytes())
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    send.write_all(bytes)
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    Ok(())
}

async fn read_prefixed(
    recv: &mut iroh::endpoint::RecvStream,
    limit: usize,
) -> Result<Vec<u8>, NetError> {
    let mut len_buf = [0_u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|_| NetError::BadMedia)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > limit {
        return Err(NetError::BadMedia);
    }
    let mut body = vec![0_u8; len];
    recv.read_exact(&mut body)
        .await
        .map_err(|_| NetError::BadMedia)?;
    Ok(body)
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Side streams that are not video, input, or clipboard.
//!
//! A side stream starts with the big-endian magic `CBDF` (`0x43424446`),
//! which is larger than a video header or an input frame. Then one kind
//! byte and one length-prefixed body. Clipboard streams are unchanged.
//! File, audio, and feedback use this framing so they are not parsed as
//! input or as a picture.

use cbdesk_proto::{
    Feedback, FileChunk, InputEvent, ProtoMessage, VideoFrameHeader, CONTROL_MAX_BYTES,
    FILE_MESSAGE_MAX_BYTES,
};

use crate::error::NetError;
use crate::media::{self, read_view_frame_after};
use crate::quic::QuicConn;

/// First four bytes of a file, audio, or feedback stream.
pub const SIDE_MAGIC: u32 = 0x4342_4446;

/// A file chunk. The host authorizes `FileWrite` or `FileRead`.
pub const KIND_FILE: u8 = 1;

/// One Opus packet. The sender authorizes `Audio`.
pub const KIND_AUDIO: u8 = 2;

/// Receiver feedback for the bitrate adapter. Requires `View`.
pub const KIND_FEEDBACK: u8 = 3;

/// Empty note that the host accepted one input event. The viewer waits for
/// this before it drops the connection, so the last event is not discarded.
pub const KIND_INPUT_ACK: u8 = 4;

/// Empty note that the host finished one file. The viewer waits for this
/// before it drops the connection, so the last chunk is not discarded.
pub const KIND_FILE_ACK: u8 = 5;

const AUDIO_MAX: usize = 4 * 1024;

/// What a viewer reads from one unidirectional stream.
#[derive(Debug)]
pub enum UniBody {
    /// A video frame.
    Video(VideoFrameHeader, Vec<u8>),
    /// A file chunk.
    File(FileChunk),
    /// An Opus packet.
    Audio(Vec<u8>),
    /// A bitrate report.
    Feedback(Feedback),
    /// The host accepted one input event.
    InputAck,
    /// The host finished one file.
    FileAck,
}

/// What the host reads from one unidirectional stream.
#[derive(Debug)]
pub(crate) enum HostUni {
    /// An input event.
    Input(InputEvent),
    /// A file chunk.
    File(FileChunk),
    /// An Opus packet.
    Audio(Vec<u8>),
    /// A bitrate report.
    Feedback(Feedback),
}

/// Read the next viewer stream: a frame, or a side message.
pub async fn recv_uni(conn: &QuicConn) -> Result<UniBody, NetError> {
    let mut recv = conn.accept_uni_stream().await?;
    let word = read_word(&mut recv).await?;
    if word == SIDE_MAGIC {
        return read_side(&mut recv).await;
    }
    let (header, payload) = read_view_frame_after(&mut recv, word).await?;
    Ok(UniBody::Video(header, payload))
}

pub(crate) async fn read_host_uni(
    recv: &mut iroh::endpoint::RecvStream,
) -> Result<HostUni, NetError> {
    let word = read_word(recv).await?;
    if word == SIDE_MAGIC {
        return match read_side(recv).await? {
            UniBody::File(chunk) => Ok(HostUni::File(chunk)),
            UniBody::Audio(packet) => Ok(HostUni::Audio(packet)),
            UniBody::Feedback(feedback) => Ok(HostUni::Feedback(feedback)),
            UniBody::Video(_, _) | UniBody::InputAck | UniBody::FileAck => Err(NetError::BadMedia),
        };
    }
    Ok(HostUni::Input(media::read_input_after(recv, word).await?))
}

/// Send one file chunk on its own stream.
pub async fn send_file_chunk(conn: &QuicConn, chunk: &FileChunk) -> Result<(), NetError> {
    let bytes = chunk.to_bytes().map_err(|_| NetError::BadMedia)?;
    send_side(conn, KIND_FILE, &bytes, FILE_MESSAGE_MAX_BYTES).await
}

/// Send one Opus packet on its own stream.
pub async fn send_audio_packet(conn: &QuicConn, packet: &[u8]) -> Result<(), NetError> {
    send_side(conn, KIND_AUDIO, packet, AUDIO_MAX).await
}

/// Tell the viewer that one input event was accepted.
pub async fn send_input_ack(conn: &QuicConn) -> Result<(), NetError> {
    send_side(conn, KIND_INPUT_ACK, &[], 0).await
}

/// Tell the viewer that one file was renamed into the granted root.
pub async fn send_file_ack(conn: &QuicConn) -> Result<(), NetError> {
    send_side(conn, KIND_FILE_ACK, &[], 0).await
}

/// Send one feedback report on its own stream.
pub async fn send_feedback(conn: &QuicConn, feedback: &Feedback) -> Result<(), NetError> {
    let bytes = feedback.to_bytes().map_err(|_| NetError::BadMedia)?;
    send_side(conn, KIND_FEEDBACK, &bytes, CONTROL_MAX_BYTES).await
}

async fn send_side(conn: &QuicConn, kind: u8, body: &[u8], limit: usize) -> Result<(), NetError> {
    if body.len() > limit {
        return Err(NetError::BadMedia);
    }
    let mut send = conn.open_uni_stream().await?;
    send.write_all(&SIDE_MAGIC.to_be_bytes())
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    send.write_all(&[kind])
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    let len = u32::try_from(body.len()).map_err(|_| NetError::BadMedia)?;
    send.write_all(&len.to_be_bytes())
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    send.write_all(body)
        .await
        .map_err(|err| NetError::Quic(err.to_string()))?;
    send.finish()
        .map_err(|err| NetError::Quic(err.to_string()))?;
    Ok(())
}

async fn read_side(recv: &mut iroh::endpoint::RecvStream) -> Result<UniBody, NetError> {
    let mut kind = [0_u8; 1];
    recv.read_exact(&mut kind)
        .await
        .map_err(|_| NetError::BadMedia)?;
    let limit = match kind[0] {
        KIND_FILE => FILE_MESSAGE_MAX_BYTES,
        KIND_AUDIO => AUDIO_MAX,
        KIND_FEEDBACK => CONTROL_MAX_BYTES,
        KIND_INPUT_ACK | KIND_FILE_ACK => 0,
        _ => return Err(NetError::BadMedia),
    };
    let body = read_body(recv, limit).await?;
    match kind[0] {
        KIND_FILE => Ok(UniBody::File(
            FileChunk::from_bytes(&body).map_err(|_| NetError::BadMedia)?,
        )),
        KIND_AUDIO => Ok(UniBody::Audio(body)),
        KIND_FEEDBACK => Ok(UniBody::Feedback(
            Feedback::from_bytes(&body).map_err(|_| NetError::BadMedia)?,
        )),
        KIND_INPUT_ACK if body.is_empty() => Ok(UniBody::InputAck),
        KIND_FILE_ACK if body.is_empty() => Ok(UniBody::FileAck),
        _ => Err(NetError::BadMedia),
    }
}

async fn read_word(recv: &mut iroh::endpoint::RecvStream) -> Result<u32, NetError> {
    let mut buf = [0_u8; 4];
    recv.read_exact(&mut buf)
        .await
        .map_err(|_| NetError::BadMedia)?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_body(
    recv: &mut iroh::endpoint::RecvStream,
    limit: usize,
) -> Result<Vec<u8>, NetError> {
    let word = read_word(recv).await? as usize;
    if word > limit {
        return Err(NetError::BadMedia);
    }
    let mut body = vec![0_u8; word];
    recv.read_exact(&mut body)
        .await
        .map_err(|_| NetError::BadMedia)?;
    Ok(body)
}

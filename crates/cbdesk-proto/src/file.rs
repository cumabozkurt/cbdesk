// SPDX-License-Identifier: AGPL-3.0-or-later

//! One chunk of a file transfer.
//!
//! The bytes do not authorize themselves. The host still has to have granted
//! [`crate::Permission::FileWrite`] to accept an upload, or
//! [`crate::Permission::FileRead`] to send one. `compressed` is false in this
//! build: zstd is not linked. A receiver rejects a chunk that says otherwise.

use crate::error::ProtoError;
use crate::wire::{self, Fields, UnknownFields};
use crate::ProtoMessage;

/// Largest chunk body this message will carry.
pub const FILE_CHUNK_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Largest encoding of [`FileChunk`], body plus protobuf overhead.
pub const FILE_MESSAGE_MAX_BYTES: usize = FILE_CHUNK_MAX_BYTES + 64 * 1024;

/// One BLAKE3-addressed slice of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChunk {
    /// Sender-local id shared by every chunk of this file.
    pub transfer: u64,
    /// Path relative to the granted root. The receiver jails it.
    pub path: String,
    /// Byte offset of `data` in the file.
    pub offset: u64,
    /// Chunk bytes. Empty is a zero-length file when `last` is set.
    pub data: Vec<u8>,
    /// BLAKE3 of `data`. Empty input still has a 32-byte hash.
    pub hash: Vec<u8>,
    /// Final file size in bytes.
    pub total: u64,
    /// True on the chunk that ends at `total`.
    pub last: bool,
    /// BLAKE3 of the whole file. Present when `last` is set.
    pub root: Vec<u8>,
    /// True when `data` is zstd. This build rejects that.
    pub compressed: bool,
    /// Chunk size the sender is using, in bytes.
    pub chunk_size: u64,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for FileChunk {
    fn default() -> Self {
        Self {
            transfer: 0,
            path: String::new(),
            offset: 0,
            data: Vec::new(),
            hash: Vec::new(),
            total: 0,
            last: false,
            root: Vec::new(),
            compressed: false,
            chunk_size: 0,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for FileChunk {
    const MAX_BYTES: usize = FILE_MESSAGE_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        check_chunk(self)?;
        wire::write_u64(out, 1, self.transfer);
        wire::write_string(out, 2, &self.path);
        wire::write_u64(out, 3, self.offset);
        wire::write_bytes(out, 4, &self.data);
        wire::write_bytes(out, 5, &self.hash);
        wire::write_u64(out, 6, self.total);
        wire::write_bool(out, 7, self.last);
        wire::write_bytes(out, 8, &self.root);
        wire::write_bool(out, 9, self.compressed);
        wire::write_u64(out, 10, self.chunk_size);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        let chunk = Self {
            transfer: fields.take_u64(1)?,
            path: fields.take_string(2)?,
            offset: fields.take_u64(3)?,
            data: fields.take_bytes(4)?,
            hash: fields.take_bytes(5)?,
            total: fields.take_u64(6)?,
            last: fields.take_bool(7)?,
            root: fields.take_bytes(8)?,
            compressed: fields.take_bool(9)?,
            chunk_size: fields.take_u64(10)?,
            unknown: fields.finish_unknown(),
        };
        check_chunk(&chunk)?;
        Ok(chunk)
    }
}

fn check_chunk(chunk: &FileChunk) -> Result<(), ProtoError> {
    if chunk.data.len() > FILE_CHUNK_MAX_BYTES {
        return Err(ProtoError::TooLarge {
            len: chunk.data.len(),
            limit: FILE_CHUNK_MAX_BYTES,
        });
    }
    if chunk.hash.len() != 32 || (chunk.last && chunk.root.len() != 32) {
        return Err(ProtoError::TooLarge {
            len: chunk.hash.len().saturating_add(chunk.root.len()),
            limit: 32,
        });
    }
    if !chunk.last && !chunk.root.is_empty() {
        return Err(ProtoError::TooLarge {
            len: chunk.root.len(),
            limit: 0,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunk_round_trips() {
        let message = FileChunk {
            transfer: 7,
            path: "notes/üğişçö.txt".to_string(),
            offset: 8,
            data: b"hello".to_vec(),
            hash: vec![1; 32],
            total: 13,
            last: true,
            root: vec![2; 32],
            compressed: false,
            chunk_size: 8,
            unknown: UnknownFields::new(),
        };
        let bytes = message.to_bytes().expect("encode");
        assert_eq!(FileChunk::from_bytes(&bytes).expect("decode"), message);
    }

    #[test]
    fn a_short_hash_is_rejected() {
        let message = FileChunk {
            hash: vec![1; 16],
            ..FileChunk::default()
        };
        assert!(message.to_bytes().is_err());
    }
}

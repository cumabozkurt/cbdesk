// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Resumable file transfer inside one granted directory.
//!
//! Chunks are BLAKE3-addressed. A `.cbdesk-part` file and a bitmap record
//! what has arrived, so a later session can skip those offsets. The path
//! jail rejects `..`, absolute paths, Windows reserved names, and symlinks.
//! zstd is not linked. A chunk marked compressed is refused by the caller.

mod error;
mod jail;
mod transfer;

pub use error::FsError;
pub use jail::resolve;
pub use transfer::{
    clamp_chunk_size, hash_bytes, hash_file, plan_chunks, IncomingFile, PlannedChunk,
    CHUNK_DEFAULT, CHUNK_MAX, CHUNK_MIN,
};

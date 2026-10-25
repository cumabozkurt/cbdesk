// SPDX-License-Identifier: AGPL-3.0-or-later

//! Failures from a jailed file transfer.

use thiserror::Error;

/// A chunk or a path was refused.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FsError {
    /// The path left the granted root, or it was absolute.
    #[error("path is not inside the granted root")]
    Traversal,
    /// The name is reserved or not a single path component.
    #[error("file name is not allowed")]
    Name,
    /// A symlink was in the path. This build does not follow links.
    #[error("path contains a symlink")]
    Symlink,
    /// The final file is already there.
    #[error("file already exists")]
    Exists,
    /// The chunk hash or the root hash did not match.
    #[error("file hash did not match")]
    Hash,
    /// `compressed` was set. This build does not link zstd.
    #[error("compressed chunks are not accepted")]
    Compressed,
    /// Offset, length, or chunk size did not match the transfer.
    #[error("chunk does not match the transfer")]
    Chunk,
    /// A directory was required and something else was there.
    #[error("a directory was required")]
    NotDir,
    /// The operating system refused the file.
    #[error("file transfer failed: {0}")]
    Io(String),
}

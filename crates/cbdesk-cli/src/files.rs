// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jailed uploads and the chunks a viewer sends.
//!
//! The receiver writes `.cbdesk-part` until the root BLAKE3 matches, then
//! renames the file. zstd is not linked, so a compressed chunk is refused.
//! The CLI reads at most 32 MiB so a mistaken path does not load a disk.
//! The library still resumes a transfer that is larger than one read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cbdesk_fs::{hash_bytes, plan_chunks, IncomingFile, CHUNK_DEFAULT};
use cbdesk_proto::{FileChunk, UnknownFields};

use crate::error::CliError;

/// Largest file `send_file` will read into memory.
const CLI_FILE_MAX: usize = 32 * 1024 * 1024;

/// What one accepted chunk did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stored {
    /// More chunks are still expected.
    Part,
    /// The file was renamed into the granted root.
    Done,
}

/// Open transfers under one granted directory.
pub struct Uploads {
    root: PathBuf,
    open: HashMap<u64, IncomingFile>,
}

impl Uploads {
    /// Create the directory if it is missing.
    pub fn create(root: &Path) -> Result<Self, CliError> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            open: HashMap::new(),
        })
    }

    /// Accept one chunk. A compressed body is refused before it touches disk.
    pub fn push(&mut self, chunk: &FileChunk) -> Result<Stored, CliError> {
        if chunk.compressed {
            return Err(CliError::new("compressed file chunks are not accepted"));
        }
        self.ensure(chunk)?;
        let file = self
            .open
            .get_mut(&chunk.transfer)
            .ok_or_else(|| CliError::new("file transfer is not open"))?;
        file.push(chunk.offset, &chunk.data, &chunk.hash)
            .map_err(|err| CliError::new(err.to_string()))?;
        if !chunk.last {
            return Ok(Stored::Part);
        }
        let file = self
            .open
            .remove(&chunk.transfer)
            .ok_or_else(|| CliError::new("file transfer is not open"))?;
        file.finish(&chunk.root)
            .map_err(|err| CliError::new(err.to_string()))?;
        Ok(Stored::Done)
    }

    fn ensure(&mut self, chunk: &FileChunk) -> Result<(), CliError> {
        if self.open.contains_key(&chunk.transfer) {
            return Ok(());
        }
        let file = IncomingFile::begin(&self.root, &chunk.path, chunk.total, chunk.chunk_size)
            .map_err(|err| CliError::new(err.to_string()))?;
        self.open.insert(chunk.transfer, file);
        Ok(())
    }
}

/// The chunks a viewer sends. Callers can interleave them with frame reads.
pub fn load_chunks(path: &Path) -> Result<Vec<FileChunk>, CliError> {
    let name = file_name(path)?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > CLI_FILE_MAX {
        return Err(CliError::new("file is larger than 32 MiB"));
    }
    let root = hash_bytes(&bytes);
    let planned =
        plan_chunks(&bytes, CHUNK_DEFAULT).map_err(|err| CliError::new(err.to_string()))?;
    let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut out = Vec::with_capacity(planned.len());
    for piece in planned {
        out.push(FileChunk {
            transfer: 1,
            path: name.clone(),
            offset: piece.offset,
            data: piece.data.to_vec(),
            hash: piece.hash.to_vec(),
            total,
            last: piece.last,
            root: if piece.last {
                root.to_vec()
            } else {
                Vec::new()
            },
            compressed: false,
            chunk_size: CHUNK_DEFAULT,
            unknown: UnknownFields::new(),
        });
    }
    Ok(out)
}

fn file_name(path: &Path) -> Result<String, CliError> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err(CliError::new("file name is not a single path component"));
    };
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(':') {
        return Err(CliError::new("file name is not a single path component"));
    }
    Ok(name.to_string())
}

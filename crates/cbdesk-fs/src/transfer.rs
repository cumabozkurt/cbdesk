// SPDX-License-Identifier: AGPL-3.0-or-later

//! Resumable chunks beside a `.cbdesk-part` file.
//!
//! The sender this crate recommends uses 1 MiB chunks, clamped into
//! 256 KiB..=4 MiB. Tests inside this crate may use a smaller size. The
//! part file plus a bitmap is the resume record. zstd is not applied;
//! `compressed` must stay false.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::error::FsError;
use crate::jail::{self, resolve};

/// Chunk size the command-line sender uses.
pub const CHUNK_DEFAULT: u64 = 1024 * 1024;

/// Smallest chunk the sender will choose.
pub const CHUNK_MIN: u64 = 256 * 1024;

/// Largest chunk the sender or the receiver will accept.
pub const CHUNK_MAX: u64 = 4 * 1024 * 1024;

const META_MAGIC: &[u8; 4] = b"CBDP";

/// One slice of a file, borrowed from the source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedChunk<'a> {
    /// Offset in the file.
    pub offset: u64,
    /// Chunk bytes.
    pub data: &'a [u8],
    /// BLAKE3 of `data`.
    pub hash: [u8; 32],
    /// True when this slice ends the file.
    pub last: bool,
}

/// BLAKE3 of `bytes`.
pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

/// BLAKE3 of a file, read in pieces.
pub fn hash_file(path: &Path) -> Result<[u8; 32], FsError> {
    let mut file = File::open(path).map_err(|err| FsError::Io(err.to_string()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|err| FsError::Io(err.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(*hasher.finalize().as_bytes())
}

/// Clamp a sender's chunk size into the plan's adaptive range.
pub fn clamp_chunk_size(wanted: u64) -> u64 {
    wanted.clamp(CHUNK_MIN, CHUNK_MAX)
}

/// Split `bytes` into chunks of `chunk_size`.
///
/// An empty input yields one empty last chunk. `chunk_size` must be in
/// 1..=[`CHUNK_MAX`] while this crate is tested, and in
/// [`CHUNK_MIN`]..=[`CHUNK_MAX`] for every other caller.
pub fn plan_chunks(bytes: &[u8], chunk_size: u64) -> Result<Vec<PlannedChunk<'_>>, FsError> {
    check_size(chunk_size)?;
    if bytes.is_empty() {
        return Ok(vec![PlannedChunk {
            offset: 0,
            data: &[],
            hash: hash_bytes(&[]),
            last: true,
        }]);
    }
    let mut out = Vec::new();
    let mut offset = 0_u64;
    while offset < bytes.len() as u64 {
        let len = chunk_size.min((bytes.len() as u64) - offset) as usize;
        let start = offset as usize;
        let data = &bytes[start..start + len];
        let last = offset + len as u64 == bytes.len() as u64;
        out.push(PlannedChunk {
            offset,
            data,
            hash: hash_bytes(data),
            last,
        });
        offset += len as u64;
    }
    Ok(out)
}

/// A file being received under a granted root.
pub struct IncomingFile {
    root: PathBuf,
    relative: String,
    full: PathBuf,
    part: PathBuf,
    meta: PathBuf,
    total: u64,
    chunk_size: u64,
    bitmap: Vec<u8>,
    file: Option<File>,
}

impl IncomingFile {
    /// Open or resume `relative` under `root`.
    pub fn begin(
        root: &Path,
        relative: &str,
        total: u64,
        chunk_size: u64,
    ) -> Result<Self, FsError> {
        check_size(chunk_size)?;
        let full = resolve(root, relative)?;
        if full.symlink_metadata().is_ok() {
            return Err(FsError::Exists);
        }
        jail::ensure_parent(root, &full)?;
        let part = sibling(&full, "cbdesk-part");
        let meta = sibling(&full, "cbdesk-meta");
        let chunks = chunk_count(total, chunk_size)?;
        let bitmap = load_or_new(&meta, total, chunk_size, chunks)?;
        let file = if total == 0 {
            None
        } else {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&part)
                .map_err(|err| FsError::Io(err.to_string()))?;
            file.set_len(total)
                .map_err(|err| FsError::Io(err.to_string()))?;
            Some(file)
        };
        Ok(Self {
            root: root.to_path_buf(),
            relative: relative.to_string(),
            full,
            part,
            meta,
            total,
            chunk_size,
            bitmap,
            file,
        })
    }

    /// Resume an existing transfer, or begin one.
    pub fn open(root: &Path, relative: &str, total: u64, chunk_size: u64) -> Result<Self, FsError> {
        Self::begin(root, relative, total, chunk_size)
    }

    /// Write one chunk whose BLAKE3 is `hash`.
    pub fn push(&mut self, offset: u64, data: &[u8], hash: &[u8]) -> Result<(), FsError> {
        if hash != hash_bytes(data) {
            return Err(FsError::Hash);
        }
        if self.total == 0 {
            if offset == 0 && data.is_empty() {
                return Ok(());
            }
            return Err(FsError::Chunk);
        }
        if self.chunk_size == 0 || !offset.is_multiple_of(self.chunk_size) {
            return Err(FsError::Chunk);
        }
        let index = offset / self.chunk_size;
        let expected = if offset + (data.len() as u64) == self.total {
            (self.total - offset) as usize
        } else {
            self.chunk_size as usize
        };
        if data.len() != expected || offset + (data.len() as u64) > self.total {
            return Err(FsError::Chunk);
        }
        let Some(file) = self.file.as_mut() else {
            return Err(FsError::Chunk);
        };
        file.seek(SeekFrom::Start(offset))
            .map_err(|err| FsError::Io(err.to_string()))?;
        file.write_all(data)
            .map_err(|err| FsError::Io(err.to_string()))?;
        set_bit(&mut self.bitmap, index)?;
        write_meta(&self.meta, self.total, self.chunk_size, &self.bitmap)?;
        Ok(())
    }

    /// Offsets that have not been written.
    pub fn missing(&self) -> Vec<u64> {
        let mut out = Vec::new();
        let chunks = chunk_count(self.total, self.chunk_size).unwrap_or(0);
        for index in 0..chunks {
            if !bit(&self.bitmap, index) {
                out.push(index.saturating_mul(self.chunk_size));
            }
        }
        out
    }

    /// True when every chunk bit is set.
    pub fn is_complete(&self) -> bool {
        self.missing().is_empty()
    }

    /// Check the root hash, rename the part file, and delete the bitmap.
    pub fn finish(mut self, root_hash: &[u8]) -> Result<PathBuf, FsError> {
        if !self.is_complete() {
            return Err(FsError::Chunk);
        }
        if self.total == 0 {
            if root_hash != hash_bytes(&[]) {
                return Err(FsError::Hash);
            }
            std::fs::write(&self.full, []).map_err(|err| FsError::Io(err.to_string()))?;
            let _ = std::fs::remove_file(&self.meta);
            return Ok(self.full);
        }
        let Some(mut file) = self.file.take() else {
            return Err(FsError::Chunk);
        };
        file.seek(SeekFrom::Start(0))
            .map_err(|err| FsError::Io(err.to_string()))?;
        let mut hasher = blake3::Hasher::new();
        let mut buf = [0_u8; 64 * 1024];
        let mut left = self.total;
        while left > 0 {
            let want = (buf.len() as u64).min(left) as usize;
            file.read_exact(&mut buf[..want])
                .map_err(|err| FsError::Io(err.to_string()))?;
            hasher.update(&buf[..want]);
            left -= want as u64;
        }
        drop(file);
        if hasher.finalize().as_bytes() != root_hash {
            return Err(FsError::Hash);
        }
        std::fs::rename(&self.part, &self.full).map_err(|err| FsError::Io(err.to_string()))?;
        let _ = std::fs::remove_file(&self.meta);
        Ok(self.full)
    }

    /// Granted root this transfer is jailed to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Relative name the sender asked for.
    pub fn relative(&self) -> &str {
        &self.relative
    }
}

fn check_size(chunk_size: u64) -> Result<(), FsError> {
    let min = if cfg!(test) { 1 } else { CHUNK_MIN };
    if chunk_size < min || chunk_size > CHUNK_MAX {
        Err(FsError::Chunk)
    } else {
        Ok(())
    }
}

fn chunk_count(total: u64, chunk_size: u64) -> Result<u64, FsError> {
    if total == 0 {
        return Ok(0);
    }
    if chunk_size == 0 {
        return Err(FsError::Chunk);
    }
    Ok(total.div_ceil(chunk_size))
}

fn sibling(full: &Path, suffix: &str) -> PathBuf {
    let mut name = full
        .file_name()
        .map(|part| part.to_os_string())
        .unwrap_or_default();
    name.push(".");
    name.push(suffix);
    match full.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

fn load_or_new(meta: &Path, total: u64, chunk_size: u64, chunks: u64) -> Result<Vec<u8>, FsError> {
    let fresh = vec![0_u8; bitmap_len(chunks)];
    if !meta.exists() {
        write_meta(meta, total, chunk_size, &fresh)?;
        return Ok(fresh);
    }
    let bytes = std::fs::read(meta).map_err(|err| FsError::Io(err.to_string()))?;
    let (seen_total, seen_size, bitmap) = parse_meta(&bytes).ok_or(FsError::Chunk)?;
    if seen_total != total || seen_size != chunk_size || bitmap.len() != fresh.len() {
        return Err(FsError::Chunk);
    }
    Ok(bitmap)
}

fn bitmap_len(chunks: u64) -> usize {
    (chunks.saturating_add(7) / 8) as usize
}

fn write_meta(path: &Path, total: u64, chunk_size: u64, bitmap: &[u8]) -> Result<(), FsError> {
    let mut bytes = Vec::with_capacity(4 + 1 + 8 + 8 + 4 + bitmap.len());
    bytes.extend_from_slice(META_MAGIC);
    bytes.push(1);
    bytes.extend_from_slice(&total.to_le_bytes());
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    let len = u32::try_from(bitmap.len()).map_err(|_| FsError::Chunk)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(bitmap);
    std::fs::write(path, bytes).map_err(|err| FsError::Io(err.to_string()))
}

fn parse_meta(bytes: &[u8]) -> Option<(u64, u64, Vec<u8>)> {
    if bytes.len() < 25 || &bytes[0..4] != META_MAGIC || bytes[4] != 1 {
        return None;
    }
    let total = u64::from_le_bytes(bytes[5..13].try_into().ok()?);
    let chunk_size = u64::from_le_bytes(bytes[13..21].try_into().ok()?);
    let len = u32::from_le_bytes(bytes[21..25].try_into().ok()?) as usize;
    let bitmap = bytes.get(25..25 + len)?.to_vec();
    Some((total, chunk_size, bitmap))
}

fn set_bit(bitmap: &mut [u8], index: u64) -> Result<(), FsError> {
    let byte = usize::try_from(index / 8).map_err(|_| FsError::Chunk)?;
    let slot = bitmap.get_mut(byte).ok_or(FsError::Chunk)?;
    *slot |= 1 << (index % 8);
    Ok(())
}

fn bit(bitmap: &[u8], index: u64) -> bool {
    let Some(byte) = usize::try_from(index / 8).ok() else {
        return false;
    };
    bitmap
        .get(byte)
        .is_some_and(|slot| slot & (1 << (index % 8)) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_finishes() {
        let root = tempfile::tempdir().expect("root");
        let mut incoming = IncomingFile::begin(root.path(), "empty.bin", 0, 8).expect("begin");
        incoming.push(0, &[], &hash_bytes(&[])).expect("push");
        let path = incoming.finish(&hash_bytes(&[])).expect("finish");
        assert_eq!(std::fs::read(&path).expect("read"), b"");
    }

    #[test]
    fn a_partial_chunk_resumes() {
        let root = tempfile::tempdir().expect("root");
        let bytes = b"0123456789abcdef";
        let mut incoming = IncomingFile::begin(root.path(), "data.bin", 16, 8).expect("begin");
        incoming
            .push(0, &bytes[..8], &hash_bytes(&bytes[..8]))
            .expect("first");
        assert_eq!(incoming.missing(), vec![8]);
        drop(incoming);
        let mut incoming = IncomingFile::begin(root.path(), "data.bin", 16, 8).expect("resume");
        assert_eq!(incoming.missing(), vec![8]);
        incoming
            .push(8, &bytes[8..], &hash_bytes(&bytes[8..]))
            .expect("second");
        let path = incoming.finish(&hash_bytes(bytes)).expect("finish");
        assert_eq!(std::fs::read(&path).expect("read"), bytes);
        assert!(!root.path().join("data.bin.cbdesk-part").exists());
    }

    #[test]
    fn a_unicode_name_round_trips() {
        let root = tempfile::tempdir().expect("root");
        let bytes = "merhaba".as_bytes();
        let mut incoming =
            IncomingFile::begin(root.path(), "üğişçö.txt", bytes.len() as u64, 8).expect("begin");
        incoming.push(0, bytes, &hash_bytes(bytes)).expect("push");
        let path = incoming.finish(&hash_bytes(bytes)).expect("finish");
        assert!(path.ends_with("üğişçö.txt"));
        assert_eq!(std::fs::read(&path).expect("read"), bytes);
    }

    #[test]
    fn the_sender_range_clamps() {
        assert_eq!(clamp_chunk_size(1), CHUNK_MIN);
        assert_eq!(clamp_chunk_size(CHUNK_DEFAULT), CHUNK_DEFAULT);
        assert_eq!(clamp_chunk_size(u64::MAX), CHUNK_MAX);
    }
}

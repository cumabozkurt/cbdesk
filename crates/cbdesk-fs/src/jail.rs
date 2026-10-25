// SPDX-License-Identifier: AGPL-3.0-or-later

//! Keep a relative path inside one granted root.
//!
//! `..`, absolute paths, Windows reserved device names, and symlinks are
//! refused. The check does not follow a link to see where it points.

use std::path::{Path, PathBuf};

use crate::error::FsError;

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Resolve `name` under `root`.
///
/// `name` uses `/` between components on every operating system. `\` and `:`
/// are refused so a Windows separator cannot sneak through on another host.
pub fn resolve(root: &Path, name: &str) -> Result<PathBuf, FsError> {
    let relative = relative_path(name)?;
    let mut full = root.to_path_buf();
    if link_or_missing_parent(&full)? {
        return Err(FsError::Symlink);
    }
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(FsError::Traversal);
        };
        full.push(part);
        match full.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => return Err(FsError::Symlink),
            Ok(meta) if meta.is_file() && full != root.join(&relative) => {
                return Err(FsError::NotDir);
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => break,
            Err(err) => return Err(FsError::Io(err.to_string())),
        }
    }
    Ok(root.join(relative))
}

/// Create parent directories of `full` without following a symlink.
pub fn ensure_parent(root: &Path, full: &Path) -> Result<(), FsError> {
    let relative = full.strip_prefix(root).map_err(|_| FsError::Traversal)?;
    let mut cursor = root.to_path_buf();
    reject_link(&cursor)?;
    let parent = relative.parent().unwrap_or(Path::new(""));
    for component in parent.components() {
        cursor.push(component);
        match cursor.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => return Err(FsError::Symlink),
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(FsError::NotDir),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&cursor).map_err(|err| FsError::Io(err.to_string()))?;
            }
            Err(err) => return Err(FsError::Io(err.to_string())),
        }
    }
    Ok(())
}

fn relative_path(name: &str) -> Result<PathBuf, FsError> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\0')
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(FsError::Traversal);
    }
    let mut path = PathBuf::new();
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(FsError::Traversal);
        }
        check_component(part)?;
        path.push(part);
    }
    Ok(path)
}

fn check_component(part: &str) -> Result<(), FsError> {
    if part.len() > 255 || part.ends_with('.') || part.ends_with(' ') {
        return Err(FsError::Name);
    }
    if part.chars().any(|ch| ch.is_control()) {
        return Err(FsError::Name);
    }
    let stem = part.split('.').next().unwrap_or(part);
    if RESERVED.iter().any(|item| stem.eq_ignore_ascii_case(item)) {
        return Err(FsError::Name);
    }
    Ok(())
}

fn link_or_missing_parent(path: &Path) -> Result<bool, FsError> {
    match path.symlink_metadata() {
        Ok(meta) => Ok(meta.file_type().is_symlink()),
        Err(err) => Err(FsError::Io(err.to_string())),
    }
}

fn reject_link(path: &Path) -> Result<(), FsError> {
    if link_or_missing_parent(path)? {
        Err(FsError::Symlink)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_segments_and_reserved_names_are_rejected() {
        let root = tempfile::tempdir().expect("root");
        assert!(matches!(
            resolve(root.path(), "../secret"),
            Err(FsError::Traversal)
        ));
        assert!(matches!(
            resolve(root.path(), "/etc/passwd"),
            Err(FsError::Traversal)
        ));
        assert!(matches!(resolve(root.path(), "CON"), Err(FsError::Name)));
        assert!(matches!(
            resolve(root.path(), "aux.txt"),
            Err(FsError::Name)
        ));
        let unicode = resolve(root.path(), "notes/üğişçö.txt").expect("unicode");
        assert!(unicode.ends_with("üğişçö.txt"));
        let emoji = resolve(root.path(), "📄.txt").expect("emoji");
        assert!(emoji.ends_with("📄.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_component_is_rejected() {
        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).expect("link");
        assert!(matches!(
            resolve(root.path(), "link/secret.txt"),
            Err(FsError::Symlink)
        ));
    }
}

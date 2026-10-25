#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Pack an unsigned archive and append a sha256 line.

This script does not sign. It does not download dependencies.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import tarfile
import zipfile
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def append_sum(sums: Path, filename: str, digest: str) -> None:
    lines = []
    if sums.is_file():
        for line in sums.read_text(encoding="utf-8").splitlines():
            if line.endswith(f"  {filename}"):
                continue
            if line.strip():
                lines.append(line)
    lines.append(f"{digest}  {filename}")
    sums.write_text("\n".join(lines) + "\n", encoding="utf-8")


def add_tree(destination: Path, source: Path, prefix: str, fmt: str) -> None:
    if fmt == "tar.gz":
        with tarfile.open(destination, "w:gz") as archive:
            _add_tar_path(archive, source, prefix)
        return
    if fmt == "zip":
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            _add_zip_path(archive, source, prefix)
        return
    raise SystemExit(f"unknown format {fmt}")


def _add_tar_path(archive: tarfile.TarFile, source: Path, prefix: str) -> None:
    if source.is_file():
        info = archive.gettarinfo(source, prefix)
        info.mode = 0o755 if os.access(source, os.X_OK) else 0o644
        with source.open("rb") as handle:
            archive.addfile(info, handle)
        return
    for path in sorted(source.rglob("*")):
        relative = path.relative_to(source).as_posix()
        arcname = f"{prefix}/{relative}" if prefix else relative
        if path.is_symlink():
            info = tarfile.TarInfo(arcname)
            info.type = tarfile.SYMTYPE
            info.linkname = os.readlink(path)
            archive.addfile(info)
            continue
        if path.is_dir():
            info = tarfile.TarInfo(arcname)
            info.type = tarfile.DIRTYPE
            info.mode = 0o755
            archive.addfile(info)
            continue
        if path.is_file():
            info = archive.gettarinfo(path, arcname)
            info.mode = 0o755 if os.access(path, os.X_OK) else 0o644
            with path.open("rb") as handle:
                archive.addfile(info, handle)


def _add_zip_path(archive: zipfile.ZipFile, source: Path, prefix: str) -> None:
    if source.is_file():
        archive.write(source, prefix)
        return
    for path in sorted(source.rglob("*")):
        relative = path.relative_to(source).as_posix()
        arcname = f"{prefix}/{relative}" if prefix else relative
        if path.is_dir():
            archive.write(path, arcname + "/")
            continue
        if path.is_symlink() or path.is_file():
            archive.write(path, arcname)


def main() -> None:
    parser = argparse.ArgumentParser(description="Pack an unsigned archive and record sha256.")
    sub = parser.add_subparsers(dest="command", required=True)

    pack = sub.add_parser("pack")
    pack.add_argument("--output", type=Path, required=True)
    pack.add_argument("--format", choices=("tar.gz", "zip"), required=True)
    pack.add_argument("--prefix", required=True, help="Path of the payload inside the archive")
    pack.add_argument("--input", type=Path, required=True, help="File or directory to pack")
    pack.add_argument("--note", type=Path, help="Extra text file stored next to the payload")
    pack.add_argument("--sums", type=Path, help="SHA256SUMS file to append")

    record = sub.add_parser("sum")
    record.add_argument("--file", type=Path, required=True)
    record.add_argument("--sums", type=Path, required=True)

    args = parser.parse_args()
    if args.command == "sum":
        if not args.file.is_file():
            raise SystemExit(f"missing file: {args.file}")
        append_sum(args.sums, args.file.name, sha256(args.file))
        return

    if not args.input.exists():
        raise SystemExit(f"missing input: {args.input}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    stage = args.output.parent / f".{args.output.name}.stage"
    if stage.exists():
        raise SystemExit(f"stage already exists: {stage}")
    stage.mkdir()
    try:
        payload = stage / args.prefix
        if args.input.is_dir():
            payload.mkdir()
            add_tree_copy(args.input, payload)
        else:
            payload.parent.mkdir(parents=True, exist_ok=True)
            payload.write_bytes(args.input.read_bytes())
            payload.chmod(0o755 if os.access(args.input, os.X_OK) else 0o644)
        if args.note is not None:
            note_dest = stage / args.note.name
            note_dest.write_bytes(args.note.read_bytes())
        add_tree(args.output, stage, "", args.format)
    finally:
        remove_tree(stage)
    if args.sums is not None:
        append_sum(args.sums, args.output.name, sha256(args.output))
    print(f"wrote {args.output}")


def add_tree_copy(source: Path, destination: Path) -> None:
    for path in sorted(source.rglob("*")):
        relative = destination / path.relative_to(source)
        if path.is_dir():
            relative.mkdir(parents=True, exist_ok=True)
            continue
        relative.parent.mkdir(parents=True, exist_ok=True)
        if path.is_symlink():
            relative.symlink_to(os.readlink(path))
            continue
        relative.write_bytes(path.read_bytes())
        if os.access(path, os.X_OK):
            relative.chmod(0o755)


def remove_tree(path: Path) -> None:
    if not path.exists():
        return
    for child in sorted(path.rglob("*"), reverse=True):
        if child.is_symlink() or child.is_file():
            child.unlink()
        elif child.is_dir():
            child.rmdir()
    path.rmdir()


if __name__ == "__main__":
    main()

<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Release scripts

These scripts do not put a private key in git. `.github/workflows/release.yml`
calls them on a `v*` tag and on a manual run that names a tag. This tree
does not contain that tag, and these scripts have not published a file.

- `release-tag.sh` accepts a `v*` tag and rejects anything else, so the
  value can be used in a filename.
- `pack.py` writes a `.tar.gz` or `.zip` and can append a sha256 line.
  It does not sign. macOS `.app` bundles are packed with `ditto` in the
  workflow, not with this script.
- `UNSIGNED.txt` is copied into the CLI and desktop archives.
- `release-notes.md` is the prerelease text. `@@TAG@@` is replaced with
  the tag. The notes say the files are an unsigned alpha, that SmartScreen
  and Gatekeeper can warn or block them, and that no real-device or
  performance test was recorded.
- `sbom.py` writes a package list from `cargo metadata`.
- `sign.sh` writes a sha256 file. When `CBDESK_SIGN_KEY` is unset it writes
  a note that starts with `unsigned`.
- `reproducible.sh` prints build inputs. It does not compare two binaries.

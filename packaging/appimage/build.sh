#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Pack an AppImage when linuxdeploy is on PATH. This script does not download it.
set -eu
if ! command -v linuxdeploy >/dev/null 2>&1; then
  echo "appimage: linuxdeploy is not installed; no image was built" >&2
  exit 1
fi
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build --locked --release -p cbdesk-cli --bin cbdesk
linuxdeploy --appdir AppDir --executable target/release/cbdesk --output appimage
echo "appimage: linuxdeploy finished"

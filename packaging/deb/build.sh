#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Build an unsigned deb from the release cbdesk binary.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$root"
version=${CBDESK_VERSION:-0.0.0}
if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "dpkg-deb is not installed" >&2
  exit 1
fi
if [ "${CBDESK_REUSE_BINARY:-}" != 1 ]; then
  cargo build --locked --release -p cbdesk-cli --bin cbdesk
fi
if [ ! -f target/release/cbdesk ]; then
  echo "missing target/release/cbdesk" >&2
  exit 1
fi
arch=$(dpkg --print-architecture)
stage=$(mktemp -d)
cleanup() { rm -rf "$stage"; }
trap cleanup EXIT
install -D "target/release/cbdesk" "$stage/usr/bin/cbdesk"
mkdir -p "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<EOF
Package: cbdesk
Version: ${version}
Section: net
Priority: optional
Architecture: ${arch}
Maintainer: CBDesk contributors <https://github.com/cumabozkurt/cbdesk>
Description: Consented remote desktop
 CBDesk asks before a session and keeps an indicator that cannot be turned off.
 This package is unsigned.
EOF
out="cbdesk_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$stage" "$out"
echo "wrote $out"

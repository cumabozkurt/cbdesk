#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Template for an unsigned DMG. It runs only on macOS and does not notarize.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
  echo "dmg: hdiutil runs on macOS" >&2
  exit 1
fi
if [ "$#" -ne 1 ]; then
  echo "usage: packaging/macos/make-dmg.sh APP_BUNDLE" >&2
  exit 2
fi
app=$1
stage=$(mktemp -d)
cp -R "$app" "$stage/"
hdiutil create -volname CBDesk -srcfolder "$stage" -ov cbdesk-unsigned.dmg
rm -rf "$stage"
echo "wrote cbdesk-unsigned.dmg"

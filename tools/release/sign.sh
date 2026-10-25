#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Checksum a file. Sign it only when CBDESK_SIGN_KEY names a gpg key.
# This script does not contain a private key.
set -eu
if [ "$#" -ne 1 ]; then
  echo "usage: tools/release/sign.sh FILE" >&2
  exit 2
fi
file=$1
if [ ! -f "$file" ]; then
  echo "missing file: $file" >&2
  exit 1
fi
sha256sum "$file" > "$file.sha256"
if [ -z "${CBDESK_SIGN_KEY:-}" ]; then
  printf '%s\n' "unsigned: CBDESK_SIGN_KEY is unset" > "$file.sign-note"
  echo "unsigned: wrote $file.sha256"
  exit 0
fi
if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg is not installed" >&2
  exit 1
fi
gpg --batch --yes --local-user "$CBDESK_SIGN_KEY" --detach-sign --output "$file.sig" "$file"
echo "signed: $file.sig"

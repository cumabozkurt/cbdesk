#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Accept a GitHub release tag and print it. Reject anything that is not a
# simple v* tag so the value can be used in a filename.
set -eu
tag=${1:-}
printf '%s\n' "$tag" | grep -Eq '^v[0-9A-Za-z._-]+$' || {
  echo "tag must look like v0.1.0-alpha.1" >&2
  exit 1
}
printf '%s\n' "$tag"

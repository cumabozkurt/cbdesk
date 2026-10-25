#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Record the inputs a second machine needs. This script does not compare binaries.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$root"
if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
  SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
  export SOURCE_DATE_EPOCH
fi
export CARGO_INCREMENTAL=0
echo "SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
rustc -vV
sha256sum Cargo.lock
echo "remap suggestion: RUSTFLAGS=--remap-path-prefix=${root}=/cbdesk"
echo "A second build of this commit has to be hashed before this tree can say the binary matches."

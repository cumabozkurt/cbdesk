#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Fail when the other project's name appears outside the attribution allowlist.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"

allow='
docs/MASTER_PLAN.md
docs/adr/002-fork-sequencing.md
docs/adr/015-clean-room-media.md
README.md
NOTICE
CHANGELOG.md
tools/rebrand-check.sh
docs/dev/upstream-sync.md
docs/dev/m0-status.md
'

fail=0
files=$(git grep -I -i -l -e rustdesk -- . || true)
for file in $files; do
    case "$allow" in
        *"
$file
"*) ;;
        *)
            echo "rebrand: disallowed mention in $file" >&2
            fail=1
            ;;
    esac
done

rust_hits=$(git grep -I -i -l -e rustdesk -- '*.rs' || true)
if [ -n "$rust_hits" ]; then
    echo "rebrand: Rust sources must not name the other project" >&2
    printf '%s\n' "$rust_hits" >&2
    fail=1
fi

exit "$fail"

#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Every non-merge commit since the base revision needs a Signed-off-by trailer.
set -eu
base=${1:-origin/main}
if ! git rev-parse --verify "$base" >/dev/null 2>&1; then
    echo "DCO: base revision $base was not found" >&2
    exit 1
fi

missing=0
commits=$(git rev-list --no-merges "$base..HEAD")
for commit in $commits; do
    if ! git log -1 --format=%B "$commit" | grep -q '^Signed-off-by: '; then
        echo "DCO: missing Signed-off-by on $commit" >&2
        missing=1
    fi
done
exit "$missing"

#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# The profiles are documentation. This does not create network namespaces.
set -eu
dir=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
for name in N0 N3 N4; do
    file=$dir/profiles/$name.txt
    if [ ! -f "$file" ]; then
        echo "netlab: missing $file" >&2
        exit 1
    fi
    grep -q "^name: $name$" "$file"
    grep -q "^nat: " "$file"
    grep -q "^status: skeleton$" "$file"
done
echo "netlab: N0 N3 N4 skeletons ok"

#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Build the bilingual user guide into ./site.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$root"
command -v mdbook >/dev/null
out=$root/site
rm -rf "$out"
mkdir -p "$out/en" "$out/tr"
mdbook build "$root/docs/book/en"
mdbook build "$root/docs/book/tr"
cp -a "$root/docs/book/en/book/." "$out/en/"
cp -a "$root/docs/book/tr/book/." "$out/tr/"
cp "$root/docs/book/home/index.html" "$out/index.html"
echo "book: wrote $out"

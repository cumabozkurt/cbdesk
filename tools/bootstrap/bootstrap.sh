#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Check the local toolchain. This script does not install services.
set -eu
if ! command -v rustc >/dev/null 2>&1; then
    echo "rustc is not installed. Install Rust 1.83 with rustup, then re-run." >&2
    exit 1
fi
version=$(rustc --version)
echo "$version"
case "$version" in
    *" 1.83."*) ;;
    *)
        echo "warning: rust-toolchain.toml pins 1.83. rustup will download it on the next cargo command if rustup is installed." >&2
        ;;
esac
echo "Next: cargo test --workspace --all-targets"

# SPDX-License-Identifier: AGPL-3.0-or-later
# Check the local toolchain. This script does not install services.
$ErrorActionPreference = 'Stop'
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Error 'rustc is not installed. Install Rust 1.83 with rustup, then re-run.'
}
$version = rustc --version
Write-Output $version
if ($version -notmatch ' 1\.83\.') {
    Write-Warning 'rust-toolchain.toml pins 1.83. rustup will download it on the next cargo command if rustup is installed.'
}
Write-Output 'Next: cargo test --workspace --all-targets'

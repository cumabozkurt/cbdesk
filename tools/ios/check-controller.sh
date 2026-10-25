#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Type-check the view-side controller for the iOS device target.
# A signed TestFlight build is not produced here.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
  echo "ios: aarch64-apple-ios check runs on macOS"
  if [ "${CBDESK_IOS_REQUIRED:-}" = "1" ]; then
    exit 1
  fi
  exit 0
fi
if ! xcrun --sdk iphoneos --show-sdk-path >/dev/null 2>&1; then
  echo "ios: the iphoneos SDK is not installed" >&2
  exit 1
fi
rustup target add aarch64-apple-ios
cargo check -p cbdesk-controller --target aarch64-apple-ios
echo "ios: cbdesk-controller type-checks for aarch64-apple-ios"

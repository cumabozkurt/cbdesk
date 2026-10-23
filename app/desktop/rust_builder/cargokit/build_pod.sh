#!/bin/sh
set -e

BASEDIR=$(dirname "$0")

# Workaround for https://github.com/dart-lang/pub/issues/4010
BASEDIR=$(cd "$BASEDIR" ; pwd -P)

# Remove XCode SDK from path. Otherwise this breaks tool compilation when building iOS project
NEW_PATH=`echo $PATH | tr ":" "\n" | grep -v "Contents/Developer/" | tr "\n" ":"`

export PATH=${NEW_PATH%?} # remove trailing :

env

# Platform name (macosx, iphoneos, iphonesimulator)
export CARGOKIT_DARWIN_PLATFORM_NAME=$PLATFORM_NAME

# Arctive architectures (arm64, armv7, x86_64), space separated.
export CARGOKIT_DARWIN_ARCHS=$ARCHS

# Current build configuration (Debug, Release)
export CARGOKIT_CONFIGURATION=$CONFIGURATION

# Path to directory containing Cargo.toml.
# An absolute argument is the crate directory. A relative one stays
# relative to the pod, which is what upstream cargokit does.
case "$1" in
  /*) CARGOKIT_MANIFEST_DIR="$1" ;;
  *) CARGOKIT_MANIFEST_DIR="$PODS_TARGET_SRCROOT/$1" ;;
esac
# BASEDIR is the physical cargokit directory (pwd -P above). The podspec
# argument can still name a path that climbed the ephemeral symlink.
if [ ! -f "$CARGOKIT_MANIFEST_DIR/Cargo.toml" ]; then
  _alt="$(cd "$BASEDIR/../../../.." && pwd -P)/crates/cbdesk-ffi"
  if [ -f "$_alt/Cargo.toml" ]; then
    CARGOKIT_MANIFEST_DIR="$_alt"
  fi
fi
if [ -d "$CARGOKIT_MANIFEST_DIR" ]; then
  CARGOKIT_MANIFEST_DIR="$(cd "$CARGOKIT_MANIFEST_DIR" && pwd -P)"
fi
echo "cbdesk_ffi crate: $CARGOKIT_MANIFEST_DIR"
if [ ! -f "$CARGOKIT_MANIFEST_DIR/Cargo.toml" ]; then
  echo "cbdesk_ffi: Cargo.toml not found at $CARGOKIT_MANIFEST_DIR" >&2
  exit 1
fi
export CARGOKIT_MANIFEST_DIR

# Temporary directory for build artifacts.
export CARGOKIT_TARGET_TEMP_DIR=$TARGET_TEMP_DIR

# Output directory for final artifacts.
export CARGOKIT_OUTPUT_DIR=$PODS_CONFIGURATION_BUILD_DIR/$PRODUCT_NAME

# Directory to store built tool artifacts.
export CARGOKIT_TOOL_TEMP_DIR=$TARGET_TEMP_DIR/build_tool

# Directory inside root project. Not necessarily the top level directory of root project.
export CARGOKIT_ROOT_PROJECT_DIR=$SRCROOT

FLUTTER_EXPORT_BUILD_ENVIRONMENT=(
  "$PODS_ROOT/../Flutter/ephemeral/flutter_export_environment.sh" # macOS
  "$PODS_ROOT/../Flutter/flutter_export_environment.sh" # iOS
)

for path in "${FLUTTER_EXPORT_BUILD_ENVIRONMENT[@]}"
do
  if [[ -f "$path" ]]; then
    source "$path"
  fi
done

sh "$BASEDIR/run_build_tool.sh" build-pod "$@"

# Make a symlink from built framework to phony file, which will be used as input to
# build script. This should force rebuild (podspec currently doesn't support alwaysOutOfDate
# attribute on custom build phase)
ln -fs "$OBJROOT/XCBuildData/build.db" "${BUILT_PRODUCTS_DIR}/cargokit_phony"
ln -fs "${BUILT_PRODUCTS_DIR}/${EXECUTABLE_PATH}" "${BUILT_PRODUCTS_DIR}/cargokit_phony_out"

#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Talk to a USB Android device when one is attached.
# CBDESK_ANDROID_DEVICE=1 makes a missing device a failure.
# CBDESK_ANDROID_APK, when set, is installed and the activity is started.
set -eu
required=0
if [ "${CBDESK_ANDROID_DEVICE:-}" = "1" ]; then
  required=1
fi
if ! command -v adb >/dev/null 2>&1; then
  echo "android: adb is not installed; no device was tested"
  if [ "$required" = 1 ]; then
    exit 1
  fi
  exit 0
fi
devices=$(adb devices | awk 'NR>1 && $2=="device" { print $1 }')
if [ -z "$devices" ]; then
  echo "android: adb has no device in the device state"
  if [ "$required" = 1 ]; then
    exit 1
  fi
  exit 0
fi
echo "android: attached"
# shellcheck disable=SC2086
adb -s $(printf '%s\n' $devices | head -n 1) shell getprop ro.product.model
if [ -n "${CBDESK_ANDROID_APK:-}" ]; then
  serial=$(printf '%s\n' $devices | head -n 1)
  adb -s "$serial" install -r "$CBDESK_ANDROID_APK"
  adb -s "$serial" shell am start -n org.cbdesk.cbdesk/.MainActivity
  echo "android: started org.cbdesk.cbdesk/.MainActivity"
fi

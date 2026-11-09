#!/usr/bin/env bash
# Builds the TV app on this machine: unit tests, a signed release APK and a debug APK, under the
# shared build lock and CPU/memory cap (see AGENTS.md), and copies the release APK to
# /root/librarian-android-dist. Usage: android-tv/build.sh [extra gradle tasks]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
export ANDROID_HOME="${ANDROID_HOME:-/opt/android-sdk}"
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}"
flock /root/.librarian-build.lock systemd-run --scope -q -p CPUQuota=320% -p MemoryMax=6G \
  --setenv=ANDROID_HOME="$ANDROID_HOME" --setenv=JAVA_HOME="$JAVA_HOME" --working-directory="$here" \
  "$here/gradlew" --no-daemon --console=plain testDebugUnitTest distApks "$@"
version="$(sed -n 's/^versionName=//p' "$here/version.properties")"
mkdir -p /root/librarian-android-dist
cp "$here/app/build/dist/librarian-tv-$version.apk" /root/librarian-android-dist/
ls -l "/root/librarian-android-dist/librarian-tv-$version.apk"
sha256sum "/root/librarian-android-dist/librarian-tv-$version.apk"

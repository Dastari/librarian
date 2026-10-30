#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# All builds share one quota, including Gradle's JVMs, aapt2, javac, dexing, and lint.
# Application servers and interactive tools stay in their existing cgroups.
build_group="${LIBRARIAN_BUILD_CGROUP:-/sys/fs/cgroup/librarian-builds}"
if [[ ! -f /sys/fs/cgroup/cgroup.controllers ]]; then
    echo "This launcher requires Linux cgroup v2. See README.md for other build environments." >&2
    exit 1
fi
mkdir -p "$build_group"
build_quota="$(python3 - <<'PY'
import os
from pathlib import Path
cpus = float(len(os.sched_getaffinity(0)))
quota, period = Path('/sys/fs/cgroup/cpu.max').read_text().split()
if quota != 'max':
    cpus = min(cpus, int(quota) / int(period))
print(max(1000, int(cpus * 0.8 * 100000)))
PY
)"
printf '%s 100000\n' "$build_quota" > "$build_group/cpu.max"
printf '%s\n' "$BASHPID" > "$build_group/cgroup.procs"
if [[ $# == 0 ]]; then
    set -- assembleDebug testDebugUnitTest lintDebug
fi
exec ./gradlew --no-daemon "$@"

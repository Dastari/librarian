#!/usr/bin/env bash
# Tooling SDL only. Never runs a Librarian server or qualifies runtime bodies.
set -euo pipefail
if [[ $# != 2 ]]; then
  echo "Usage: $0 <existing-build-cgroup> <sdl-output>" >&2
  exit 2
fi
build_group=$(realpath "$1")
output=$(realpath -m "$2")
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ ! -w "$build_group/cgroup.procs" || ! -r "$build_group/cpu.max" ]]; then
  echo "A writable aggregate build cgroup with cpu.max is required." >&2
  exit 2
fi
read -r quota period < "$build_group/cpu.max"
maximum_quota=$(( $(nproc) * period * 4 / 5 ))
if [[ "$quota" == max ]] || (( quota > maximum_quota )); then
  echo "The aggregate build cgroup must cap compiler/linker children at 80% of available CPUs or less." >&2
  exit 2
fi
cd "$repo_root/backend"
(
  echo "$BASHPID" > "$build_group/cgroup.procs"
  export CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 RUSTC_BOOTSTRAP=1
  export RUSTC_WORKSPACE_WRAPPER="$repo_root/scripts/schema-export-rustc.py"
  export MALLOC_ARENA_MAX=1
  export MALLOC_CONF=dirty_decay_ms:0,muzzy_decay_ms:0,narenas:1
  export _RJEM_MALLOC_CONF="$MALLOC_CONF"
  exec timeout --signal=TERM 600 cargo build --locked --example export-schema --features schema-export-only -j 1
)
(
  echo "$BASHPID" > "$build_group/cgroup.procs"
  exec timeout 90 ./target/debug/examples/export-schema "$output"
)
echo "Tooling SDL exported to $output; normal backend compilation, tests and SDL parity remain separate gates." >&2

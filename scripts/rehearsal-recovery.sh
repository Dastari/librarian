#!/usr/bin/env bash
# Parent-owned compile lane. This script never starts Librarian services or deletes data.
set -euo pipefail
if (( $# < 2 || $# > 3 )); then
  echo "Usage: $0 <existing-build-cgroup> <new-absolute-run-directory> [--omit-public-dispatch]" >&2
  exit 2
fi
dispatch_omitted=false
if [[ $# == 3 ]]; then
  if [[ "$3" != --omit-public-dispatch ]]; then
    echo "Unknown rehearsal option: $3" >&2
    exit 2
  fi
  dispatch_omitted=true
fi
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
build_group=$(realpath "$1")
run_root=$2
if [[ "$run_root" != /* || -e "$run_root" || -L "$run_root" ]]; then
  echo "The rehearsal directory must be an absolute, nonexistent path; existing targets are never reused." >&2
  exit 2
fi
if [[ ! -w "$build_group/cgroup.procs" || ! -r "$build_group/cpu.max" ]]; then
  echo "A writable parent-owned aggregate build cgroup is required." >&2
  exit 2
fi
read -r quota period < "$build_group/cpu.max"
maximum_quota=$(( $(nproc) * period * 4 / 5 ))
if [[ "$quota" == max ]] || (( quota <= 0 || period <= 0 || quota > maximum_quota )); then
  echo "The aggregate build cgroup must cap all compiler/linker children at 80% of available CPUs or less." >&2
  exit 2
fi
# Exclusive mkdir reserves this run; failure preserves everything for inspection.
mkdir -- "$run_root"
run_root=$(realpath "$run_root")
mkdir -- "$run_root/logs" "$run_root/bin" "$run_root/state"

python3 - "$repo_root" "$run_root" "$dispatch_omitted" <<'PY'
import hashlib
import json
from pathlib import Path
import shutil
import sys

repo, run = map(Path, sys.argv[1:3])
dispatch_omitted = sys.argv[3] == "true"
stage = run / "source"
stage.mkdir()
for relative in ["backend/src", "backend/tests/fixtures", "backend/examples/rehearse", "vendor"]:
    shutil.copytree(repo / relative, stage / relative)
helpers = ["backend/Cargo.toml", "backend/Cargo.lock", "scripts/rehearsal-rustc.py"]
if dispatch_omitted:
    helpers += ["scripts/rehearsal-dispatch.rs", "scripts/rehearsal-dispatch-stage.py"]
for relative in helpers:
    destination = stage / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(repo / relative, destination)
# Snapshot all copied input bytes before adding the two explicit harness bridges.
files = {str(path.relative_to(stage)): hashlib.sha256(path.read_bytes()).hexdigest()
         for path in sorted(stage.rglob("*")) if path.is_file()}
(run / "input-sha256.json").write_text(json.dumps(files, indent=2) + "\n")
lib = stage / "backend/src/lib.rs"
lib.write_text(lib.read_text() + '\n#[path = "../examples/rehearse/recovery.rs"]\npub mod recovery_rehearsal;\n')
backup = stage / "backend/src/services/backup.rs"
backup.write_text(backup.read_text() + '\n#[path = "../../examples/rehearse/fault.rs"]\nmod rehearsal_fault;\npub(crate) use rehearsal_fault::inject_second_object_failure;\n')
(stage / "backend/examples/rehearse-recovery.rs").write_text('''
#[cfg(feature = "schema-export-only")]
compile_error!("Recovery rehearsal cannot use schema-export-only");
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    librarian::recovery_rehearsal::run().await
}
''')
# Synthetic historical schema: identical Rust/API field with an old physical name.
# This tests intentional rename-as-drop/add on fixture rows, never a real upgrade.
entity = stage / "backend/src/services/graphql/entities/app_setting.rs"
text = entity.read_text()
needle = '    pub description: Option<String>,'
if text.count(needle) != 1:
    raise SystemExit("AppSetting description contract changed; update the rehearsal explicitly")
entity.write_text(text.replace(needle, '    #[db_column = "rehearsal_description_v1"]\n' + needle))
(run / "staging-contract.json").write_text(json.dumps({
    "sourceSchemaChange": "AppSetting.description db_column=rehearsal_description_v1",
    "candidateSchemaChange": "remove only that synthetic db_column attribute",
    "runtimeBodies": "normal sqlite feature; schema-export-only disabled",
    "libraryBridge": "recovery_rehearsal module in staged lib.rs only",
    "backupBridge": "fault injection module in staged backup.rs only",
    "liveDataCopied": False,
}, indent=2) + "\n")
PY
if [[ "$dispatch_omitted" == true ]]; then
  python3 "$run_root/source/scripts/rehearsal-dispatch-stage.py" prepare "$run_root"
fi

# Reuse parent-owned dependency artifacts unless the parent requests a separate cache.
# Cargo's lock serializes this target; the parent must run no other builds concurrently.
build_target=${REHEARSAL_TARGET_DIR:-"$repo_root/backend/target"}
build_target=$(realpath -m "$build_target")
{
  echo "buildCgroup=$build_group"
  echo "buildTarget=$build_target"
  echo "publicDispatchOmitted=$dispatch_omitted"
  echo "cpu.max=$(cat "$build_group/cpu.max")"
  for controller in memory.max memory.swap.max; do
    if [[ -r "$build_group/$controller" ]]; then
      echo "$controller=$(cat "$build_group/$controller")"
    fi
  done
} > "$run_root/resources.txt"

build_variant() {
  local variant=$1
  if [[ "$dispatch_omitted" == true ]]; then
    echo "Building selected typed runtime; public dispatch omitted: $variant (log: $run_root/logs/build-$variant.log)" >&2
    python3 "$run_root/source/scripts/rehearsal-dispatch-stage.py" verify "$run_root" "$variant"
  else
    echo "Building normal runtime rehearsal: $variant (log: $run_root/logs/build-$variant.log)" >&2
  fi
  (
    echo "$BASHPID" > "$build_group/cgroup.procs"
    cd -- "$run_root/source/backend"
    unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER LIBRARIAN_REHEARSAL_DISPATCH_MARKER
    if [[ "$dispatch_omitted" == true ]]; then
      export LIBRARIAN_REHEARSAL_DISPATCH_MARKER=typed-runtime-only-v1
    fi
    export CARGO_TARGET_DIR="$build_target" CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
    export RUSTC_BOOTSTRAP=1 RUSTC_WORKSPACE_WRAPPER="$run_root/source/scripts/rehearsal-rustc.py"
    export MALLOC_ARENA_MAX=1 MALLOC_CONF=dirty_decay_ms:0,muzzy_decay_ms:0,narenas:1
    export _RJEM_MALLOC_CONF="$MALLOC_CONF"
    rustc -Vv
    exec timeout --signal=TERM --kill-after=30s 1800 cargo build --locked --offline \
      --no-default-features --features sqlite --example rehearse-recovery -j 1
  ) > "$run_root/logs/build-$variant.log" 2>&1
  if [[ "$dispatch_omitted" == true ]]; then
    python3 "$run_root/source/scripts/rehearsal-dispatch-stage.py" audit "$run_root" "$variant"
    python3 "$run_root/source/scripts/rehearsal-dispatch-stage.py" verify "$run_root" "$variant"
  fi
  cp -- "$build_target/debug/examples/rehearse-recovery" "$run_root/bin/$variant"
  sha256sum -- "$run_root/bin/$variant" >> "$run_root/binary-sha256.txt"
}

# Each phase is a fresh OS process outside the compilation cgroup.
phase() {
  local variant=$1 action=$2
  shift 2
  echo "Rehearsal $variant / $action $*" >&2
  env -i PATH="$PATH" HOME="$HOME" \
    timeout --signal=TERM --kill-after=10s 180 "$run_root/bin/$variant" "$action" "$run_root/state" "$@" \
    > "$run_root/logs/$variant-$action${1:+-$1}.log" 2>&1
}

build_variant source-schema
for action in seed summaries backup; do phase source-schema "$action"; done
# Recovery is qualified before the candidate binary is even built.
phase source-schema restore qualified
phase source-schema verify qualified
phase source-schema partial-failure
phase source-schema restart-rejection

python3 - "$run_root/source/backend/src/services/graphql/entities/app_setting.rs" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
needle = '    #[db_column = "rehearsal_description_v1"]\n'
if text.count(needle) != 1:
    raise SystemExit("Synthetic source-schema marker changed")
path.write_text(text.replace(needle, ""))
PY
build_variant candidate-schema
phase candidate-schema plan
# Record a failed real migration gate, then still prove recovery from fresh targets.
# A blocked apply is never reported as a successful destructive migration.
migration_passed=false
if phase candidate-schema migrate; then migration_passed=true; fi
phase candidate-schema incompatible-recovery
phase source-schema restore rollback
phase source-schema verify rollback
python3 - "$run_root" "$migration_passed" "$dispatch_omitted" <<'PY'
import json
from pathlib import Path
import sys
run = Path(sys.argv[1])
result = {
    "sourceSchemaRecovery": "passed",
    "summaryDryRunRepairIdempotence": "passed",
    "partialFailureAndProcessRestartRejection": "passed",
    "candidateRejectsSourceSchemaRecovery": "passed",
    "reviewedDestructiveMigration": "passed" if sys.argv[2] == "true" else "BLOCKED: see logs/candidate-schema-migrate.log",
    "rollbackRowsAndObjectBytes": "passed",
    "wholeApplicationQualification": False,
    "actualHistoricalReleaseUpgrade": False,
    "generatedPublicDispatch": "omitted" if sys.argv[3] == "true" else "present, not exercised",
    "publicGraphqlDispatchQualified": False,
    "normalLibraryBuildCompleted": sys.argv[3] != "true",
    "qualificationScope": "selected typed recovery/reconciliation paths",
}
(run / "result.json").write_text(json.dumps(result, indent=2) + "\n")
PY
if [[ "$migration_passed" != true ]]; then
  echo "Recovery rehearsal retained at $run_root; reviewed migration is BLOCKED. See logs/candidate-schema-migrate.log and state/snapshot.json." >&2
  exit 1
fi
if [[ "$dispatch_omitted" == true ]]; then
  echo "Selected typed recovery rehearsal passed; public GraphQL dispatch omitted and normal backend build unqualified. Evidence: $run_root/result.json" >&2
else
  echo "Recovery rehearsal passed; evidence retained at $run_root/result.json" >&2
fi

#!/usr/bin/env python3
"""Prepare/check a marked rehearsal source copy. Never alter the real macro package."""
import hashlib
import json
import os
import stat
from pathlib import Path
import sys

MARKER = "typed-runtime-only-v1"
AUDIT_PREFIX = "LIBRARIAN_REHEARSAL_DISPATCH_V1\t"
LIB_BRIDGE = '\n#[path = "../examples/rehearse/recovery.rs"]\npub mod recovery_rehearsal;\n'
FAULT_BRIDGE = '\n#[path = "../../examples/rehearse/fault.rs"]\nmod rehearsal_fault;\npub(crate) use rehearsal_fault::inject_second_object_failure;\n'
SYNTHETIC_COLUMN = '    #[db_column = "rehearsal_description_v1"]\n'
RUN_NEEDLE = '#[cfg(not(feature = "schema-export-only"))]\npub fn run() -> anyhow::Result<()> {'
RUN_GUARD = '#[cfg(not(librarian_recovery_rehearsal))]\n'
LIB_GUARDS = '''
// Staged recovery qualification only. Production function bodies above remain intact.
#[cfg(not(librarian_recovery_rehearsal))]
compile_error!("This staged library requires the explicit recovery rehearsal marker");
#[cfg(any(test, doctest))]
compile_error!("Dispatch-omitted recovery cannot qualify backend tests or doctests");
#[cfg(librarian_recovery_rehearsal)]
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("Dispatch-omitted recovery cannot start Librarian services")
}
'''
MAIN_GUARD = '\ncompile_error!("This staged source copy cannot build the Librarian server binary");\n'
EXAMPLE_GUARD = '''
#[cfg(not(librarian_recovery_rehearsal))]
compile_error!("Dispatch-omitted recovery example requires the explicit rehearsal marker");
'''
WRAPPER_NEEDLE = 'if crate in {"librarian", "rehearse_recovery"}:\n'
WRAPPER_GUARD = '''if crate in {"librarian", "rehearse_recovery"}:
    if os.environ.get("LIBRARIAN_REHEARSAL_DISPATCH_MARKER") != "typed-runtime-only-v1":
        sys.exit("Staged recovery requires the explicit typed-runtime-only-v1 marker")
    if "--test" in arguments:
        sys.exit("Dispatch-omitted recovery cannot qualify backend tests")
    arguments.extend(["--cfg", "librarian_recovery_rehearsal",
                      "--check-cfg", "cfg(librarian_recovery_rehearsal)"])

'''
EXAMPLE_PATH = "backend/examples/rehearse-recovery.rs"
EXAMPLE_ENTRY = '''
#[cfg(feature = "schema-export-only")]
compile_error!("Recovery rehearsal cannot use schema-export-only");
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    librarian::recovery_rehearsal::run().await
}
'''
MACRO_PATH = "vendor/graphql-orm-macros/src/schema_export.rs"
MACRO_ORIGINAL_PATH = MACRO_PATH + ".production-original"
# These are the complete, fixed additions to the input file inventory. Neither
# arbitrary build scripts/configuration nor an altered harness can be adopted.
PREPARED_ADDITIONS = frozenset({EXAMPLE_PATH, MACRO_ORIGINAL_PATH})
CONTRACT = "dispatch-staging-contract.json"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def replace_once(text, needle, replacement, label):
    require(text.count(needle) == 1, f"{label}: expected exactly one known staging boundary")
    return text.replace(needle, replacement, 1)


def remove_suffix(text, suffix, label):
    require(text.endswith(suffix), f"{label}: required staging suffix missing or changed")
    return text[:-len(suffix)]


def file_inventory(stage):
    files = set()
    # Do not follow links or silently ignore sockets/FIFOs. A new symlinked
    # directory could otherwise hide compiler inputs from a simple rglob walk.
    def failed_walk(error):
        raise SystemExit(f"Cannot inspect complete staged file inventory: {error}")

    for directory, children, names in os.walk(stage, followlinks=False, onerror=failed_walk):
        for name in children + names:
            path = Path(directory) / name
            mode = path.lstat().st_mode
            relative = path.relative_to(stage).as_posix()
            require(not stat.S_ISLNK(mode), f"Staged inventory contains a symlink: {relative}")
            if stat.S_ISDIR(mode):
                continue
            require(stat.S_ISREG(mode), f"Staged inventory contains a special file: {relative}")
            files.add(relative)
    return files


def verify_inventory(stage, inputs, additions):
    require(not (set(inputs) & PREPARED_ADDITIONS),
            "Generated rehearsal additions unexpectedly appear in the original input manifest")
    expected = set(inputs) | set(additions)
    actual = file_inventory(stage)
    missing, extra = sorted(expected - actual), sorted(actual - expected)
    require(not missing and not extra,
            f"Staged file inventory differs: unexpected={extra}, missing={missing}")
    return actual


def verify_example(stage, guarded):
    expected = (EXAMPLE_ENTRY + (EXAMPLE_GUARD if guarded else "")).encode()
    actual = (stage / EXAMPLE_PATH).read_bytes()
    require(actual == expected,
            "Complete generated rehearsal example bytes differ from the fixed harness contract")
    return sha(expected)


def stage_paths(run):
    run = Path(run).resolve(strict=True)
    stage = run / "source"
    inputs = json.loads((run / "input-sha256.json").read_text())
    require((run / "staging-contract.json").is_file(), "Not an initialized rehearsal source copy")
    require(stage.resolve(strict=True) == stage, "Staged source root must not be a symlink")
    require((stage / "backend/src/lib.rs").is_file(), "Staged backend is missing")
    return run, stage, inputs


def prepare(run):
    run, stage, inputs = stage_paths(run)
    require(not (run / CONTRACT).exists(), "Dispatch staging is one-shot; use a fresh run directory")
    verify_inventory(stage, inputs, {EXAMPLE_PATH})
    verify_example(stage, guarded=False)
    lib = stage / "backend/src/lib.rs"
    text = lib.read_text()
    require(text.endswith(LIB_BRIDGE), "Refusing to modify a library without the isolated rehearsal bridge")
    # All writes below are confined to this already-created source copy.
    lib.write_text(replace_once(text, RUN_NEEDLE, RUN_GUARD + RUN_NEEDLE, "library run guard") + LIB_GUARDS)
    main = stage / "backend/src/main.rs"
    main.write_text(main.read_text() + MAIN_GUARD)
    example = stage / EXAMPLE_PATH
    example.write_bytes((EXAMPLE_ENTRY + EXAMPLE_GUARD).encode())
    wrapper = stage / "scripts/rehearsal-rustc.py"
    wrapper.write_text(replace_once(wrapper.read_text(), WRAPPER_NEEDLE,
                                    WRAPPER_GUARD + WRAPPER_NEEDLE, "workspace wrapper"))
    macro = stage / MACRO_PATH
    original = macro.read_bytes()
    require(sha(original) == inputs[MACRO_PATH], "Copied production macro changed before staging")
    original_path = macro.with_suffix(".rs.production-original")
    with original_path.open("xb") as output:
        output.write(original)
    helper = stage / "scripts/rehearsal-dispatch.rs"
    macro.write_bytes(helper.read_bytes())
    contract = {
        "marker": MARKER,
        "publicDispatch": "omitted",
        "normalBackendQualification": False,
        "globalCargoFeaturesAdded": False,
        "sourceBodyInvariant": "Exact file inventory, complete generated example bytes, and every input SHA-256 after reversing only documented staging edits",
        "allowedAddedFiles": sorted(PREPARED_ADDITIONS),
        "generatedExampleSha256": sha((EXAMPLE_ENTRY + EXAMPLE_GUARD).encode()),
        "expansionInvariant": "Every emitted expansion has identical normalized tokens outside allowlisted resolver bodies",
        "allowedAttributes": ["graphql_orm::async_graphql::Object", "graphql_orm::async_graphql::Subscription",
                              "graphql_orm::async_graphql::ComplexObject", "async_graphql::Object",
                              "async_graphql::Subscription", "async_graphql::ComplexObject"],
        "allowedMethods": "Direct async methods in inherent impls; skipped helpers, synchronous methods and unknown impl items reject staging",
        "productionMacroOriginalSha256": sha(original),
        "stagedMacroSha256": sha(helper.read_bytes()),
        "serverGuards": ["main.rs compile_error", "original lib.run excluded by cfg; replacement fails immediately",
                         "library and example require explicit marker", "backend tests/doctests forbidden"],
    }
    (run / CONTRACT).write_text(json.dumps(contract, indent=2) + "\n")
    base = json.loads((run / "staging-contract.json").read_text())
    base["runtimeBodies"] = "selected typed runtime intact; generated public GraphQL dispatch omitted in staged macro copy"
    base["dispatchContract"] = CONTRACT
    (run / "staging-contract.json").write_text(json.dumps(base, indent=2) + "\n")
    verify(run, "source-schema")


def verify(run, variant):
    require(variant in {"source-schema", "candidate-schema"}, "Unknown rehearsal source variant")
    run, stage, inputs = stage_paths(run)
    contract = json.loads((run / CONTRACT).read_text())
    require(contract["marker"] == MARKER, "Dispatch marker contract changed")
    # Inventory and generated content are derived from fixed code, not adopted
    # from a mutable contract's allowlist/hash fields.
    inventory = verify_inventory(stage, inputs, PREPARED_ADDITIONS)
    example_hash = verify_example(stage, guarded=True)
    require(contract["allowedAddedFiles"] == sorted(PREPARED_ADDITIONS), "Added-file contract changed")
    require(contract["generatedExampleSha256"] == example_hash, "Generated-example hash contract changed")
    checked = 0
    for relative, expected in inputs.items():
        path = stage / relative
        data = path.read_bytes()
        if relative == "backend/src/lib.rs":
            text = remove_suffix(data.decode(), LIB_GUARDS, "library guards")
            text = replace_once(text, RUN_GUARD + RUN_NEEDLE, RUN_NEEDLE, "original run bytes")
            data = remove_suffix(text, LIB_BRIDGE, "library bridge").encode()
        elif relative == "backend/src/main.rs":
            data = remove_suffix(data.decode(), MAIN_GUARD, "server build guard").encode()
        elif relative == "backend/src/services/backup.rs":
            data = remove_suffix(data.decode(), FAULT_BRIDGE, "backup fault bridge").encode()
        elif relative == "backend/src/services/graphql/entities/app_setting.rs":
            text = data.decode()
            if variant == "source-schema":
                text = replace_once(text, SYNTHETIC_COLUMN, "", "synthetic source column")
            else:
                require(SYNTHETIC_COLUMN not in text, "Candidate still has the synthetic source column")
            data = text.encode()
        elif relative == "scripts/rehearsal-rustc.py":
            data = replace_once(data.decode(), WRAPPER_GUARD + WRAPPER_NEEDLE,
                                WRAPPER_NEEDLE, "workspace marker guard").encode()
        elif relative == MACRO_PATH:
            require(data == (stage / "scripts/rehearsal-dispatch.rs").read_bytes(), "Staged resolver transform changed")
            require(sha(data) == contract["stagedMacroSha256"], "Staged transform fingerprint changed")
            data = path.with_suffix(".rs.production-original").read_bytes()
        require(sha(data) == expected, f"Input bytes changed outside the allowed staging edits: {relative}")
        checked += 1
    report = {"variant": variant, "inputFilesChecked": checked, "originalBytesPreserved": True,
              "exactFileInventory": True, "stagedFileCount": len(inventory),
              "allowedAddedFiles": sorted(PREPARED_ADDITIONS), "generatedExampleSha256": example_hash,
              "completeGeneratedExampleVerified": True,
              "dispatchOmitted": True, "qualification": "selected typed runtime paths only"}
    (run / f"source-invariant-{variant}.json").write_text(json.dumps(report, indent=2) + "\n")


def audit(run, variant):
    run, _, _ = stage_paths(run)
    selected = []
    for line in (run / "logs" / f"build-{variant}.log").read_text().splitlines():
        if line.startswith(AUDIT_PREFIX):
            fields = line.split("\t")
            require(len(fields) == 5 and fields[-1] == "unselected-normalized-tokens=identical",
                    "Malformed dispatch expansion invariant record")
            selected.append({"impl": fields[1], "attribute": fields[2], "methods": fields[3].split(","),
                             "unselectedNormalizedTokensIdentical": True})
    require(selected, "Build produced no dispatch omission audit; do not qualify this executable")
    (run / f"dispatch-audit-{variant}.json").write_text(json.dumps({
        "variant": variant, "selectedImpls": selected, "publicDispatchQualified": False,
        "normalBackendQualified": False,
    }, indent=2) + "\n")


if __name__ == "__main__":
    require(len(sys.argv) in {3, 4}, "Usage: rehearsal-dispatch-stage.py prepare|verify|audit <run-root> [variant]")
    action, run = sys.argv[1:3]
    if action == "prepare" and len(sys.argv) == 3:
        prepare(run)
    elif action in {"verify", "audit"} and len(sys.argv) == 4:
        (verify if action == "verify" else audit)(run, sys.argv[3])
    else:
        raise SystemExit("Unknown dispatch staging command")

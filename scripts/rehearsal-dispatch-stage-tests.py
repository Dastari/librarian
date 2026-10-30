#!/usr/bin/env python3
"""Static staging integrity regressions. No Cargo, Rust compiler, or runtime build."""
import ast
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
SCRIPTS = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("rehearsal_dispatch_stage", SCRIPTS / "rehearsal-dispatch-stage.py")
staging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(staging)


class StagingIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="librarian-dispatch-integrity-")
        self.addCleanup(self.temporary.cleanup)
        self.run_root = Path(self.temporary.name)
        self.stage = self.run_root / "source"
        originals = {
            "backend/src/lib.rs": staging.RUN_NEEDLE + '\n    production_runtime()\n}\n',
            "backend/src/main.rs": 'fn main() { production_server(); }\n',
            "backend/src/services/backup.rs": '// original recovery bodies\n',
            "backend/src/services/graphql/entities/app_setting.rs": '    pub description: Option<String>,\n',
            "backend/Cargo.toml": '[package]\nname="fixture"\nversion="0.0.0"\n',
            "backend/Cargo.lock": 'version = 4\n',
            staging.MACRO_PATH: '// production macro remains intact outside the staged copy\n',
        }
        for relative, text in originals.items():
            path = self.stage / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        for name in ["rehearsal-rustc.py", "rehearsal-dispatch.rs", "rehearsal-dispatch-stage.py"]:
            path = self.stage / "scripts" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((SCRIPTS / name).read_bytes())
        inputs = {path.relative_to(self.stage).as_posix(): staging.sha(path.read_bytes())
                  for path in self.stage.rglob("*") if path.is_file()}
        (self.run_root / "input-sha256.json").write_text(json.dumps(inputs))
        (self.run_root / "staging-contract.json").write_text('{}')
        lib = self.stage / "backend/src/lib.rs"
        lib.write_text(lib.read_text() + staging.LIB_BRIDGE)
        backup = self.stage / "backend/src/services/backup.rs"
        backup.write_text(backup.read_text() + staging.FAULT_BRIDGE)
        entity = self.stage / "backend/src/services/graphql/entities/app_setting.rs"
        entity.write_text(staging.SYNTHETIC_COLUMN + entity.read_text())
        example = self.stage / staging.EXAMPLE_PATH
        example.parent.mkdir(parents=True, exist_ok=True)
        example.write_text(staging.EXAMPLE_ENTRY)

    def prepare(self):
        staging.prepare(self.run_root)

    def test_runner_generates_the_complete_fixed_example(self):
        script = (SCRIPTS / "rehearsal-recovery.sh").read_text()
        first_python = script.split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0]
        candidates = [node.args[0].value for node in ast.walk(ast.parse(first_python))
                      if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                      and node.func.attr == "write_text" and node.args
                      and isinstance(node.args[0], ast.Constant) and isinstance(node.args[0].value, str)
                      and '#[tokio::main(flavor = "current_thread")]' in node.args[0].value]
        self.assertEqual(candidates, [staging.EXAMPLE_ENTRY])

    def test_exact_known_additions_and_both_variants_pass(self):
        self.prepare()
        staging.verify(self.run_root, "source-schema")
        entity = self.stage / "backend/src/services/graphql/entities/app_setting.rs"
        entity.write_text(entity.read_text().replace(staging.SYNTHETIC_COLUMN, "", 1))
        staging.verify(self.run_root, "candidate-schema")
        report = json.loads((self.run_root / "source-invariant-candidate-schema.json").read_text())
        self.assertTrue(report["exactFileInventory"])
        self.assertTrue(report["completeGeneratedExampleVerified"])
        self.assertEqual(report["allowedAddedFiles"], sorted(staging.PREPARED_ADDITIONS))

    def test_added_build_script_is_rejected_after_prepare(self):
        self.prepare()
        (self.stage / "backend/build.rs").write_text('fn main() { /* unauthorized compiler input */ }\n')
        with self.assertRaisesRegex(SystemExit, r"inventory differs.*backend/build.rs"):
            staging.verify(self.run_root, "source-schema")

    def test_added_build_script_is_rejected_before_prepare(self):
        (self.stage / "backend/build.rs").write_text('fn main() {}\n')
        with self.assertRaisesRegex(SystemExit, r"inventory differs.*backend/build.rs"):
            self.prepare()
        self.assertFalse((self.run_root / staging.CONTRACT).exists())

    def test_changed_example_body_retaining_guard_is_rejected(self):
        self.prepare()
        example = self.stage / staging.EXAMPLE_PATH
        changed = example.read_text().replace('librarian::recovery_rehearsal::run().await', 'Ok(())', 1)
        self.assertTrue(changed.endswith(staging.EXAMPLE_GUARD))
        example.write_text(changed)
        with self.assertRaisesRegex(SystemExit, "Complete generated rehearsal example bytes differ"):
            staging.verify(self.run_root, "source-schema")

    def test_changed_example_is_not_adopted_during_prepare(self):
        example = self.stage / staging.EXAMPLE_PATH
        example.write_text(example.read_text().replace('librarian::recovery_rehearsal::run().await', 'Ok(())', 1))
        with self.assertRaisesRegex(SystemExit, "Complete generated rehearsal example bytes differ"):
            self.prepare()
        self.assertFalse((self.run_root / staging.CONTRACT).exists())

    def test_added_module_or_cargo_configuration_is_rejected(self):
        self.prepare()
        for relative in ['backend/src/hidden.rs', 'backend/.cargo/config.toml']:
            with self.subTest(relative=relative):
                path = self.stage / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('// unauthorized compiler input\n')
                try:
                    with self.assertRaisesRegex(SystemExit, "inventory differs"):
                        staging.verify(self.run_root, "source-schema")
                finally:
                    path.unlink()

    def test_missing_original_file_is_rejected(self):
        self.prepare()
        (self.stage / "backend/src/main.rs").unlink()
        with self.assertRaisesRegex(SystemExit, r"inventory differs.*missing=.*backend/src/main.rs"):
            staging.verify(self.run_root, "source-schema")

    def test_symlinked_directory_is_rejected_without_following_it(self):
        self.prepare()
        outside = self.run_root / "outside"
        outside.mkdir()
        (outside / "hidden.rs").write_text('// must not be followed\n')
        (self.stage / "backend/linked").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(SystemExit, "inventory contains a symlink"):
            staging.verify(self.run_root, "source-schema")

    def test_production_original_macro_copy_cannot_change(self):
        self.prepare()
        (self.stage / staging.MACRO_ORIGINAL_PATH).write_text('// altered saved original\n')
        with self.assertRaisesRegex(SystemExit, "Input bytes changed outside the allowed staging edits"):
            staging.verify(self.run_root, "source-schema")

    def test_contract_cannot_expand_addition_allowlist(self):
        self.prepare()
        (self.stage / "backend/build.rs").write_text('fn main() {}\n')
        contract_path = self.run_root / staging.CONTRACT
        contract = json.loads(contract_path.read_text())
        contract["allowedAddedFiles"].append("backend/build.rs")
        contract_path.write_text(json.dumps(contract))
        with self.assertRaisesRegex(SystemExit, "inventory differs"):
            staging.verify(self.run_root, "source-schema")


if __name__ == "__main__":
    unittest.main()

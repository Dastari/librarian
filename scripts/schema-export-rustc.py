#!/usr/bin/env python3
"""Reduce tooling-only codegen; leave every normal runtime invocation unchanged."""
import os
import sys

arguments = sys.argv[1:]
crate = (
    arguments[arguments.index("--crate-name") + 1]
    if "--crate-name" in arguments
    else None
)
tooling = 'feature="schema-export-only"' in arguments
if tooling and crate in {"librarian", "export_schema"}:
    arguments.extend([
        "-Zthreads=1", "-Zmir-opt-level=0", "-Zmir-strip-debuginfo=all-locals",
        "-Zfewer-names=yes", "-Zno-parallel-backend", "-Ccodegen-units=1024",
        "-Cllvm-args=-threads=1", "-Zfmt-debug=none", "-Zlocation-detail=none",
        "-Cpanic=abort", "-Cdebug-assertions=no", "-Coverflow-checks=no",
    ])
    if crate == "librarian":
        # Defer unused library codegen to the small downstream exporter. It
        # still constructs the real merged schema and its type registrations.
        arguments.append("-Zhint-mostly-unused=yes")
os.execvp(arguments[0], arguments)

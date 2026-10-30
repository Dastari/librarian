#!/usr/bin/env python3
"""Defer unused library codegen, without replacing any application runtime body."""
import os
import sys

arguments = sys.argv[1:]
crate = arguments[arguments.index("--crate-name") + 1] if "--crate-name" in arguments else None
if crate in {"librarian", "rehearse_recovery"}:
    if 'feature="schema-export-only"' in arguments:
        sys.exit("Recovery rehearsal refuses schema-export-only runtime stubs")
    arguments.extend([
        "-Zthreads=1", "-Zmir-opt-level=0", "-Zmir-strip-debuginfo=all-locals",
        "-Zfewer-names=yes", "-Zno-parallel-backend", "-Ccodegen-units=1024",
        "-Cllvm-args=-threads=1",
    ])
    if crate == "librarian":
        arguments.append("-Zhint-mostly-unused=yes")
os.execvp(arguments[0], arguments)

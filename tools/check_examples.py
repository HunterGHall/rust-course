#!/usr/bin/env python3
"""Compile-checks every ```rust block in docs/ so the lessons never contain code that doesn't build.

By default a block must compile as a complete program:

  * a block with `fn main` is compiled as-is (as a test binary if it has #[test]);
  * a block with only items (structs, impls, fns, ...) gets an empty `fn main` appended.

A comment on the line directly above the fence changes the mode:

  <!-- check: wrap -->   put the block inside `fn main() { ... }` (loose statements)
                        (combine with a second word: `wrap panics`, `wrap fail`)
  <!-- check: fail -->   the block must FAIL to compile (to show a compiler error)
  <!-- check: panics -->  compiles, and with --run must panic (exit code 101)
  <!-- check: norun -->   compiles; --run does not execute it (needs arguments, exits non-zero, ...)
  <!-- check: skip -->   not checked (needs external crates, a nightly feature, ...)

Usage: python tools/check_examples.py [--run] [files...]
  --run   also execute the compiled programs (stdin closed, 10 s timeout) and
          report ones that exit non-zero, as a sanity check on the lessons' claims.
"""
import concurrent.futures
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FENCE = re.compile(r"^(`{3,})rust\s*$")
MARK = re.compile(r"^<!--\s*check:\s*([\w ]+?)\s*-->\s*$")
PRELUDE = "#![allow(unused, dead_code, unreachable_code, irrefutable_let_patterns, static_mut_refs)]\n"


def extract(path):
    blocks = []
    lines = path.read_text(encoding="utf-8").splitlines()
    i = 0
    while i < len(lines):
        m = FENCE.match(lines[i])
        if m:
            mode = set()
            if i > 0 and lines[i - 1].startswith("<!-- sync:"):
                mode = {"skip"}  # capstone files are built and tested by cargo, not here
            elif i > 0 and MARK.match(lines[i - 1]):
                mode = set(MARK.match(lines[i - 1]).group(1).split())
            start = i + 1
            j = start
            while j < len(lines) and not lines[j].startswith(m.group(1)):
                j += 1
            blocks.append((start + 1, mode, "\n".join(lines[start:j]) + "\n"))
            i = j
        i += 1
    return blocks


def build_source(mode, code):
    if "wrap" in mode:
        return PRELUDE + "fn main() {\n" + code + "}\n"
    if "fn main" in code or "#[test]" in code:
        return PRELUDE + code
    return PRELUDE + code + "\nfn main() {}\n"


def check(job):
    path, line, mode, code, run = job
    if "skip" in mode:
        return None
    src = build_source(mode, code)
    with tempfile.TemporaryDirectory() as tmp:
        rs = Path(tmp) / "example.rs"
        rs.write_text(src, encoding="utf-8")
        exe = Path(tmp) / ("example.exe" if os.name == "nt" else "example")
        cmd = ["rustc", "--edition", "2024", "-o", str(exe), str(rs)]
        if "#[test]" in code:
            cmd.insert(1, "--test")
        proc = subprocess.run(cmd, capture_output=True, text=True,
                              encoding="utf-8", errors="replace")
        where = f"{path.relative_to(ROOT)}:{line}"
        if "fail" in mode:
            if proc.returncode == 0:
                return f"{where}: expected a compile error, but it compiled"
            return None
        if proc.returncode != 0:
            errors = "\n".join(proc.stderr.splitlines()[:25])
            return f"{where}: does not compile\n{errors}"
        if run and "#[test]" not in code and "norun" not in mode:
            try:
                result = subprocess.run([str(exe)], capture_output=True, text=True, encoding="utf-8", errors="replace",
                                        stdin=subprocess.DEVNULL, timeout=10, cwd=tmp)
            except subprocess.TimeoutExpired:
                return f"{where}: timed out (10 s)"
            if "panics" in mode:
                if result.returncode != 101:
                    return f"{where}: expected a panic (exit 101), got {result.returncode}"
            elif result.returncode != 0:
                return f"{where}: exited with {result.returncode}\n{result.stderr[:400]}"
    return None


def main():
    args = sys.argv[1:]
    run = "--run" in args
    files = [Path(a).resolve() for a in args if not a.startswith("--")]
    if not files:
        files = sorted((ROOT / "docs").rglob("*.md"))

    jobs = [(f, line, mode, code, run) for f in files for (line, mode, code) in extract(f)]
    print(f"Checking {len(jobs)} code blocks from {len(files)} files...")
    failures = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        for result in pool.map(check, jobs):
            if result:
                failures.append(result)
    for failure in failures:
        print("FAIL", failure, "\n")
    print(f"{len(jobs) - len(failures)} OK, {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

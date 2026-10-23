#!/usr/bin/env python3
"""Keeps the capstone source shown in the lessons identical to capstones/.

A lesson embeds a file like this:

    <!-- sync: capstones/task-tracker/src/main.rs -->
    ```rust
    ...copied automatically...
    ```
    <!-- /sync -->

Usage: python tools/sync_capstones.py          rewrite the lessons
       python tools/sync_capstones.py --check  fail if any lesson is out of date (CI)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BLOCK = re.compile(
    r"(<!-- sync: (?P<path>\S+) -->\n)(?P<fence>`{3,})(?P<lang>\w*)\n.*?\n(?P=fence)\n(<!-- /sync -->)",
    re.S,
)


def main():
    check = "--check" in sys.argv
    stale = []
    for md in sorted((ROOT / "docs").rglob("*.md")):
        text = md.read_text(encoding="utf-8")

        def fill(m):
            source = (ROOT / m.group("path")).read_text(encoding="utf-8").rstrip("\n")
            fence = "`" * max(3, longest_run(source) + 1)
            return f"{m.group(1)}{fence}{m.group('lang')}\n{source}\n{fence}\n<!-- /sync -->"

        new = BLOCK.sub(fill, text)
        if new != text:
            stale.append(md)
            if not check:
                md.write_text(new, encoding="utf-8", newline="\n")
    for md in stale:
        print(("out of date: " if check else "updated: ") + str(md.relative_to(ROOT)))
    return 1 if (check and stale) else 0


def longest_run(text):
    runs = re.findall(r"`+", text)
    return max((len(r) for r in runs), default=0)


if __name__ == "__main__":
    sys.exit(main())

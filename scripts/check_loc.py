#!/usr/bin/env python3
"""Report source files whose effective line count exceeds a limit.

Rust files are counted after removing inline test blocks such as ``#[test]``
functions, ``#[tokio::test]`` functions, and ``#[cfg(test)]`` modules.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import sys

DEFAULT_LIMIT = 1000
EXCLUDED_DIRS = {
    ".git",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "pkg",
    "pkg-node",
    "target",
    "target-codex",
    "vendor",
}
SOURCE_EXTENSIONS = {".js", ".jsx", ".py", ".rs", ".sh", ".ts", ".tsx"}
TEST_ATTRIBUTE_RE = re.compile(r"^\s*#\[\s*(?:[A-Za-z_]\w*::)*test\b[^\]]*\]\s*$")
CFG_TEST_RE = re.compile(r"^\s*#\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]\s*$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "List source files whose effective line count exceeds the limit. "
            "Rust inline test blocks are excluded from the count."
        )
    )
    parser.add_argument("limit", nargs="?", type=int, default=DEFAULT_LIMIT)
    parser.add_argument("--ci", action="store_true", help="exit with status 1 when violations are found")
    return parser.parse_args()


def should_skip_dir(dirname: str) -> bool:
    return dirname in EXCLUDED_DIRS or dirname.startswith(".")


def iter_source_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for current_root, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if not should_skip_dir(name))
        base = Path(current_root)
        for filename in sorted(filenames):
            path = base / filename
            if path.suffix in SOURCE_EXTENSIONS:
                files.append(path)
    return files


def count_braces(line: str) -> int:
    return line.count("{") - line.count("}")


def is_rust_test_attribute(line: str) -> bool:
    return bool(TEST_ATTRIBUTE_RE.match(line) or CFG_TEST_RE.match(line))


def count_rust_lines(path: Path) -> int:
    effective_lines = 0
    skipping_item = False
    item_started = False
    brace_depth = 0

    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            if skipping_item:
                if not item_started:
                    if "{" in line:
                        item_started = True
                        brace_depth = count_braces(line)
                        if brace_depth <= 0:
                            skipping_item = False
                            item_started = False
                            brace_depth = 0
                    elif ";" in line:
                        skipping_item = False
                else:
                    brace_depth += count_braces(line)
                    if brace_depth <= 0:
                        skipping_item = False
                        item_started = False
                        brace_depth = 0
                continue

            if is_rust_test_attribute(line):
                skipping_item = True
                item_started = False
                brace_depth = 0
                continue

            effective_lines += 1

    return effective_lines


def count_lines(path: Path) -> int:
    if path.suffix == ".rs":
        return count_rust_lines(path)

    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        return sum(1 for _ in handle)


def collect_violations(root: Path, limit: int) -> list[tuple[str, int]]:
    violations: list[tuple[str, int]] = []
    for path in iter_source_files(root):
        line_count = count_lines(path)
        if line_count > limit:
            violations.append((path.relative_to(root).as_posix(), line_count))
    violations.sort(key=lambda item: (-item[1], item[0]))
    return violations


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parent.parent
    violations = collect_violations(root, args.limit)

    if not violations:
        print(f"All source files are within the {args.limit}-line limit.")
        return 0

    print(f"Found {len(violations)} file(s) exceeding {args.limit} lines:")
    print()
    print(f"{'LINES':>6}  FILE")
    print(f"{'-----':>6}  ----")
    for relative_path, line_count in violations:
        print(f"{line_count:6d}  {relative_path}")

    return 1 if args.ci else 0


if __name__ == "__main__":
    sys.exit(main())
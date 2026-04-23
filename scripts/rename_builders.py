#!/usr/bin/env python3
"""Slice C builder renames across the workspace.

- .with_quoted_clean_price(  -> .with_quoted_clean_price(
- .with_cds_quote_bp(    -> .with_cds_quote_bp(
- .cds_quote_bp   -> .cds_quote_bp                (field access)
- "cds_quote_bp"  -> "cds_quote_bp"               (JSON test fixtures / keys)
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[1]
ROOTS = [
    REPO / "finstack",
    REPO / "finstack-py",
    REPO / "finstack-wasm",
    REPO / "scripts",
]

# (regex pattern, replacement) pairs applied in order.
REPLACEMENTS = [
    # Builder method calls. Careful: with_spread_bp must not match with_spread_bps.
    (re.compile(r"\.with_clean_price\("), ".with_quoted_clean_price("),
    (re.compile(r"\.with_cds_quote_bp\b(?!s)"), ".with_cds_quote_bp"),
    # Field access. Use word boundary to avoid matching inside other identifiers.
    (re.compile(r"\bquoted_spread_bp\b"), "cds_quote_bp"),
]

# Files whose JSON test fixtures should keep the legacy key for serde-alias
# compatibility testing. Add to skiplist if needed.
SKIP = set()

changed = []
for root in ROOTS:
    if not root.exists():
        continue
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if "target" in path.parts or ".git" in path.parts or "pkg" in path.parts:
            continue
        if path.suffix not in {".rs", ".py", ".pyi", ".json", ".toml", ".md"}:
            continue
        if str(path) in SKIP:
            continue
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            continue
        new = text
        total = 0
        for pat, repl in REPLACEMENTS:
            new, n = pat.subn(repl, new)
            total += n
        if new != text:
            path.write_text(new)
            changed.append((path, total))

for path, n in changed:
    print(f"{n}  {path.relative_to(REPO)}")
print(f"Total files changed: {len(changed)}; total replacements: {sum(n for _, n in changed)}")

"""Fix mis-attached docstrings in `.pyi` stub files.

Problem
-------
Many of the stubs in `finstack-py/finstack/**/*.pyi` use the pattern:

    def foo(...) -> T: ...
    \"\"\"Docstring...\"\"\"

In Python, that docstring is *not* attached to the function; it's just a
standalone expression in the surrounding scope. VS Code/Pylance therefore
doesn't show hover documentation for the function, and tools like `ast.get_docstring`
won't associate examples with the symbol.

This script rewrites those cases to:

    def foo(...) -> T:
        \"\"\"Docstring...\"\"\"
        ...

It is intentionally conservative:
- Only rewrites docstrings that immediately follow a `def ...: ...` stub
  (allowing blank lines in between).
- Does not try to rewrite variable docstrings (e.g. `X: int` + triple-quoted block).
- Does not attempt to handle docstrings that don't start at the same indentation
  level as the preceding `def`.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import sys

PYI_DEF_END_RE = re.compile(r"^(?P<indent>[ \t]*)def\b")
PYI_DEF_ELLIPSIS_RE = re.compile(r"^(?P<prefix>.*?):\s*\.\.\.\s*$")
TRIPLE_QUOTE_RE = re.compile(r'^(?P<indent>[ \t]*)(?P<quote>"""|\'\'\')')


@dataclass(frozen=True)
class DocBlock:
    """A triple-quoted docstring block span in a `.pyi` file."""

    start: int
    end: int  # inclusive
    quote: str


def _find_docblock(lines: list[str], start_idx: int, indent: str) -> DocBlock | None:
    """If lines[start_idx] starts a triple-quoted block at `indent`, return its span."""
    m = TRIPLE_QUOTE_RE.match(lines[start_idx])
    if not m:
        return None
    if m.group("indent") != indent:
        return None
    quote = m.group("quote")
    # Single-line docstring: """foo"""
    if lines[start_idx].count(quote) >= 2:
        return DocBlock(start=start_idx, end=start_idx, quote=quote)

    # Multi-line docstring: scan for closing quote
    for j in range(start_idx + 1, len(lines)):
        if quote in lines[j]:
            return DocBlock(start=start_idx, end=j, quote=quote)
    return None


def fix_file(path: Path) -> bool:  # noqa: PLR0915
    """Rewrite the file in-place. Returns True if changes were made."""
    original = path.read_text(encoding="utf-8")
    lines = original.splitlines(keepends=True)
    changed = False

    i = 0
    out: list[str] = []

    while i < len(lines):
        line = lines[i]

        # Detect a `def ...` (possibly multi-line) and find the line that ends with `: ...`.
        m_def = PYI_DEF_END_RE.match(line)
        if not m_def:
            out.append(line)
            i += 1
            continue

        indent = m_def.group("indent")
        def_start = i
        def_end = i
        while def_end < len(lines):
            if PYI_DEF_ELLIPSIS_RE.match(lines[def_end]):
                break
            # Stop if another def/class at same or lower indent starts before we find an ellipsis line.
            if def_end > def_start and (
                lines[def_end].startswith(indent + "def ") or lines[def_end].startswith(indent + "class ")
            ):
                break
            def_end += 1

        if def_end >= len(lines) or not PYI_DEF_ELLIPSIS_RE.match(lines[def_end]):
            # Not a `def ...: ...` signature; copy through this line and continue.
            out.append(line)
            i += 1
            continue

        # Look ahead for an immediately following docblock at the same indent (allow blank lines).
        j = def_end + 1
        while j < len(lines) and lines[j].strip() == "":
            j += 1
        doc = _find_docblock(lines, j, indent) if j < len(lines) else None
        if not doc:
            # No mis-attached docstring; copy the whole def signature as-is.
            out.extend(lines[def_start : def_end + 1])
            i = def_end + 1
            continue

        # Rewrite:
        # - Change the final signature line from `...: ...` to `...:`
        # - Move docblock into the function body
        # - Add `...` body
        body_indent = indent + ("    " if "\t" not in indent else "\t")

        out.extend(lines[def_start:def_end])
        sig_last = lines[def_end]
        m_last = PYI_DEF_ELLIPSIS_RE.match(sig_last)
        if m_last is None:
            raise RuntimeError(f"internal error: expected ': ...' line in {path} at {def_end + 1}")
        out.append(m_last.group("prefix") + ":\n")

        # Docstring block, re-indented into the body
        for k in range(doc.start, doc.end + 1):
            raw = lines[k]
            # Strip exactly the outer indent (doc is at that indent by construction).
            if raw.startswith(indent):
                raw = raw[len(indent) :]
            out.append(body_indent + raw)
        if not out[-1].endswith("\n"):
            out[-1] += "\n"

        out.append(body_indent + "...\n")

        changed = True
        i = doc.end + 1

    if not changed:
        return False

    updated = "".join(out)
    if updated == original:
        return False
    path.write_text(updated, encoding="utf-8")
    return True


def main(argv: list[str]) -> int:
    """CLI entrypoint. Accepts an optional root directory (defaults to finstack stubs)."""
    root = Path(argv[1]) if len(argv) > 1 else Path("finstack-py/finstack")
    if not root.exists():
        sys.stderr.write(f"error: path does not exist: {root}\n")
        return 2

    pyi_files = sorted(root.rglob("*.pyi"))
    changed_files = 0
    for p in pyi_files:
        if fix_file(p):
            changed_files += 1

    sys.stdout.write(f"Updated {changed_files} / {len(pyi_files)} .pyi files\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

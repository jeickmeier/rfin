#!/usr/bin/env python3
"""One-shot rename: fn value(&self, <market>: &MarketContext, ...) -> fn base_value(...).

Targets only Instrument trait impls (signatures that accept a MarketContext reference
as the first non-self parameter). Leaves alone MC payoff trait (fn value(&self, currency: Currency) -> Money)
and unrelated helpers.
"""

import re
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[1]
ROOTS = [
    REPO / "finstack" / "valuations",
    REPO / "finstack" / "portfolio",
    REPO / "finstack" / "margin",
    REPO / "finstack" / "scenarios",
    REPO / "finstack-py",
    REPO / "finstack-wasm",
]

# Matches 'fn value' followed (possibly across newlines with whitespace) by
# '(... &self ... <param>: &... MarketContext'. Non-greedy across the signature.
# The MarketContext may be fully qualified.
PATTERN = re.compile(
    r"""
    \b fn \s+ value                      # 'fn value'
    (                                    # group 1: everything between 'value' and 'MarketContext'
      \s* \( \s*                         # '('
      & (?:mut\ )? self \s* ,            # '&self,' or '&mut self,'
      \s* _? \w+ \s* :                   # e.g. 'market:' or '_market:'
      \s* &
      (?:finstack_core::market_data::context::)?  # optional fully-qualified path
      MarketContext
    )
    """,
    re.VERBOSE | re.MULTILINE,
)

changed = []
for root in ROOTS:
    if not root.exists():
        continue
    for path in root.rglob("*.rs"):
        # Skip target/ and any build artifacts.
        if "target" in path.parts:
            continue
        text = path.read_text()
        new_text, n = PATTERN.subn(r"fn base_value\1", text)
        if n:
            path.write_text(new_text)
            changed.append((path, n))

for path, n in changed:
    print(f"{n}  {path.relative_to(REPO)}")
print(f"Total files changed: {len(changed)}; total replacements: {sum(n for _, n in changed)}")

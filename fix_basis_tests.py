"""Ensure basis swap test builders include a discount curve identifier."""
# ruff: noqa: I001
from __future__ import annotations

import re
import pathlib


FILES = [
    pathlib.Path("finstack/valuations/tests/instruments/basis_swap/test_basis_swap_edge_cases.rs"),
    pathlib.Path("finstack/valuations/tests/instruments/basis_swap/test_basis_swap_sensitivities.rs"),
    pathlib.Path("finstack/valuations/tests/instruments/basis_swap/test_basis_swap_par_spread.rs"),
    pathlib.Path("finstack/valuations/tests/instruments/basis_swap/test_basis_swap_theta.rs"),
    pathlib.Path("finstack/valuations/tests/instruments/basis_swap/test_basis_swap_metrics.rs"),
]

PATTERN = re.compile(r"(BasisSwap::builder\(\)[^;]+?)\.build\(\)", flags=re.DOTALL)


def add_discount_curve(match: re.Match[str]) -> str:
    """Insert a discount curve ID into builder chains that omit it."""
    builder = match.group(1)
    if ".discount_curve_id" not in builder:
        return f'{builder}\n        .discount_curve_id(CurveId::new("USD-OIS"))\n        .build()'
    return match.group(0)


def process_file(path: pathlib.Path) -> None:
    """Apply the discount-curve fix in-place for the given file."""
    content = path.read_text()
    new_content = PATTERN.sub(add_discount_curve, content)
    if new_content != content:
        path.write_text(new_content)


def main() -> None:
    """Run the fix across all configured basis swap test files."""
    for path in FILES:
        process_file(path)


if __name__ == "__main__":
    main()

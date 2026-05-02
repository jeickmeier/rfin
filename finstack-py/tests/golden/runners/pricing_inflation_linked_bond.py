"""Domain runner for `fixed_income.inflation_linked_bond` golden fixtures."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture

from .pricing_common import run_pricing_fixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one inflation-linked bond fixture through the Python bindings."""
    return run_pricing_fixture(fixture)

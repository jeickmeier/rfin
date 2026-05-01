"""Domain runner for `rates.deposit` golden fixtures."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture

from .pricing_common import run_pricing_fixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one deposit fixture through the Python bindings."""
    return run_pricing_fixture(fixture)

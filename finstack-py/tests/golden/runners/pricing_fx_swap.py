"""Domain runner for `fx.fx_swap` golden fixtures."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture

from .pricing_common import run_pricing_fixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one FX swap fixture through the Python bindings."""
    return run_pricing_fixture(fixture)

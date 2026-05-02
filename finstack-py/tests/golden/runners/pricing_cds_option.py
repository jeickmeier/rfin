"""Domain runner for `credit.cds_option` golden fixtures."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture

from .pricing_common import run_pricing_fixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one CDS option fixture through the Python bindings."""
    return run_pricing_fixture(fixture)

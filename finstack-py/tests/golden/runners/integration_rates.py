"""Domain runner for rates integration golden fixtures."""

from __future__ import annotations

from tests.golden.runners.integration_common import run_rates_integration
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable rates integration inputs before running."""
    return run_rates_integration(fixture)

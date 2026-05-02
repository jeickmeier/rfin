"""Domain runner for swaption volatility calibration golden fixtures."""

from __future__ import annotations

from tests.golden.runners.calibration_common import run_sabr_cube_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable swaption-vol calibration inputs before running."""
    return run_sabr_cube_fixture(fixture)

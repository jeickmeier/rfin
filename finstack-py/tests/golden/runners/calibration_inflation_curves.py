"""Domain runner for inflation curve calibration golden fixtures."""

from __future__ import annotations

from tests.golden.runners.calibration_common import run_curve_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable inflation curve calibration inputs before running."""
    return run_curve_fixture(fixture)

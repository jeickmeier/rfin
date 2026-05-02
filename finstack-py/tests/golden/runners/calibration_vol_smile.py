"""Domain runner for equity and FX volatility smile calibration goldens."""

from __future__ import annotations

from tests.golden.runners.calibration_common import run_vol_smile_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable vol smile calibration inputs before running."""
    return run_vol_smile_fixture(fixture)

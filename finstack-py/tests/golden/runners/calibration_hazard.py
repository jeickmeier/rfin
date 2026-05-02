"""Domain runner for credit hazard calibration golden fixtures."""

from __future__ import annotations

from tests.golden.runners.calibration_common import run_hazard_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable hazard calibration inputs before running."""
    return run_hazard_fixture(fixture)

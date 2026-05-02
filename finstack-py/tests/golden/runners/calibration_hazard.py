"""Domain runner for credit hazard calibration golden fixtures."""

from __future__ import annotations

from tests.golden.runners._placeholders import reject_flattened_outputs
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable hazard calibration inputs before running."""
    return reject_flattened_outputs("hazard calibration runner", fixture)

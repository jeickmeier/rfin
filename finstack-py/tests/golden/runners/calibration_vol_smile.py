"""Domain runner for equity and FX volatility smile calibration goldens."""

from __future__ import annotations

from tests.golden.runners._placeholders import reject_flattened_outputs
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable vol smile calibration inputs before running."""
    return reject_flattened_outputs("vol smile calibration runner", fixture)

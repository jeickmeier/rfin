"""Swaption-vol calibration goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden

pytestmark = pytest.mark.skip(
    reason="requires executable calibration inputs; current fixtures are flattened placeholders"
)


@pytest.mark.parametrize("fixture", discover_fixtures("calibration/vol"))
def test_calibration_vol(fixture: str) -> None:
    """Run every volatility calibration fixture through the Python layer."""
    run_golden(fixture)

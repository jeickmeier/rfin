"""Rates curve calibration goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden


@pytest.mark.parametrize("fixture", discover_fixtures("calibration/curves"))
def test_calibration_curves(fixture: str) -> None:
    """Run every rates curve calibration fixture through the Python layer."""
    run_golden(fixture)

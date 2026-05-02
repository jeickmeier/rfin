"""Credit hazard calibration goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden

pytestmark = pytest.mark.skip(
    reason="requires executable calibration inputs; current fixtures are flattened placeholders"
)


@pytest.mark.parametrize("fixture", discover_fixtures("calibration/hazard"))
def test_calibration_hazard(fixture: str) -> None:
    """Run every credit hazard calibration fixture through the Python layer."""
    run_golden(fixture)

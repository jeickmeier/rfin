"""Rates integration goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden


@pytest.mark.parametrize("fixture", discover_fixtures("integration"))
def test_integration_rates(fixture: str) -> None:
    """Run every rates integration fixture through the Python layer."""
    run_golden(fixture)

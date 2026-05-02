"""Analytics goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden


@pytest.mark.parametrize("fixture", discover_fixtures("analytics"))
def test_analytics(fixture: str) -> None:
    """Run every analytics fixture through the Python layer."""
    run_golden(fixture)

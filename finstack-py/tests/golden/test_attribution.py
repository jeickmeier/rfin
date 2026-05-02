"""Attribution goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, run_golden

pytestmark = pytest.mark.skip(
    reason="requires executable attribution inputs; current fixtures are flattened placeholders"
)


@pytest.mark.parametrize("fixture", discover_fixtures("attribution"))
def test_attribution(fixture: str) -> None:
    """Run every attribution fixture through the Python layer."""
    run_golden(fixture)

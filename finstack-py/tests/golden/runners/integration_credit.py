"""Domain runner for credit integration golden fixtures."""

from __future__ import annotations

from tests.golden.runners.integration_common import run_credit_integration
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable credit integration inputs before running."""
    return run_credit_integration(fixture)

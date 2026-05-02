"""Domain runner for rates integration golden fixtures."""

from __future__ import annotations

from tests.golden.runners._placeholders import reject_flattened_outputs
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable rates integration inputs before running."""
    return reject_flattened_outputs("rates integration runner", fixture)

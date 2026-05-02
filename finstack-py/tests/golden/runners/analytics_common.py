"""Domain runner for flattened analytics golden fixtures."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Return flattened analytics outputs from the fixture payload."""
    return {key: float(value) for key, value in fixture.inputs["actual_outputs"].items()}

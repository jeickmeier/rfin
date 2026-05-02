"""Domain runner for flattened analytics golden fixtures."""

from __future__ import annotations

from tests.golden.runners import reject_flattened_outputs, validate_source_validation_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run executable analytics fixtures or validate explicit source-only fixtures."""
    if validate_source_validation_fixture("analytics runner", fixture):
        references = fixture.inputs["source_validation"].get("reference_outputs", {})
        missing = [metric for metric in fixture.expected_outputs if metric not in references]
        if missing:
            msg = f"analytics source_validation.reference_outputs missing expected metrics: {missing}"
            raise ValueError(msg)
        return {}
    if "computations" not in fixture.inputs:
        return reject_flattened_outputs("analytics runner", fixture)
    msg = (
        "analytics runner found computations, but executable analytics dispatch is not wired yet; "
        "add canonical return/price inputs and API mapping before enabling this golden"
    )
    raise ValueError(msg)

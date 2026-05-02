"""Helpers for golden fixture domains that still need executable inputs."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture


def reject_flattened_outputs(runner: str, fixture: GoldenFixture) -> dict[str, float]:
    """Fail instead of echoing frozen reference outputs as actuals."""
    snapshot_hint = (
        " fixture contains inputs.actual_outputs, which is a frozen reference snapshot and not executable input."
        if "actual_outputs" in fixture.inputs
        else ""
    )
    msg = (
        f"{runner} requires executable inputs that build canonical API calls."
        f"{snapshot_hint} Replace the flattened placeholder before enabling this golden."
    )
    raise NotImplementedError(msg)

"""Per-domain Python golden runners."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture


def validate_source_validation_fixture(runner: str, fixture: GoldenFixture) -> dict[str, float] | None:
    """Return reference outputs for explicit non-executable source-validation fixtures."""
    source_validation = fixture.inputs.get("source_validation")
    if source_validation is None:
        return None
    status = source_validation.get("status")
    if status != "non_executable":
        msg = f"{runner} source_validation status must be 'non_executable', got {status!r}"
        raise ValueError(msg)
    reason = str(source_validation.get("reason", "")).strip()
    if not reason:
        msg = f"{runner} source_validation must explain why fixture is non-executable"
        raise ValueError(msg)
    if "actual_outputs" in fixture.inputs:
        msg = (
            f"{runner} source-validation fixture must not keep inputs.actual_outputs; "
            "move frozen references under source_validation.reference_outputs"
        )
        raise ValueError(msg)
    references = source_validation.get("reference_outputs")
    if not isinstance(references, dict):
        msg = f"{runner} source_validation must retain frozen references under reference_outputs"
        raise TypeError(msg)
    missing = [metric for metric in fixture.expected_outputs if metric not in references]
    if missing:
        msg = f"{runner} source_validation.reference_outputs missing expected metrics: {missing}"
        raise ValueError(msg)
    extra = [metric for metric in references if metric not in fixture.expected_outputs]
    if extra:
        msg = f"{runner} source_validation.reference_outputs contains extra metrics: {extra}"
        raise ValueError(msg)
    for metric, expected in fixture.expected_outputs.items():
        reference = references[metric]
        if isinstance(reference, bool) or not isinstance(reference, int | float):
            msg = f"{runner} source_validation.reference_outputs[{metric!r}] must be numeric"
            raise TypeError(msg)
        if float(reference) != expected:
            msg = (
                f"{runner} source_validation.reference_outputs[{metric!r}]={float(reference):.17g} "
                f"does not exactly match expected_outputs[{metric!r}]={expected:.17g}"
            )
            raise ValueError(msg)
    return {metric: float(references[metric]) for metric in fixture.expected_outputs}


def reject_flattened_outputs(runner: str, fixture: GoldenFixture) -> dict[str, float]:
    """Fail clearly when a fixture still contains frozen output snapshots."""
    snapshot_hint = (
        " fixture contains inputs.actual_outputs, which is a frozen reference snapshot and not executable input."
        if "actual_outputs" in fixture.inputs
        else ""
    )
    msg = (
        f"{runner} requires executable inputs that build canonical API calls.{snapshot_hint} "
        "Replace the flattened placeholder with product inputs before enabling this golden."
    )
    raise ValueError(msg)

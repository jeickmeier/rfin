"""Per-domain Python golden runners."""

from __future__ import annotations

from tests.golden.schema import GoldenFixture


def validate_source_validation_fixture(runner: str, fixture: GoldenFixture) -> bool:
    """Validate explicit non-executable source-validation metadata."""
    source_validation = fixture.inputs.get("source_validation")
    if source_validation is None:
        return False
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
            "expected values belong in top-level expected_outputs"
        )
        raise ValueError(msg)
    if "reference_outputs" in source_validation:
        msg = (
            f"{runner} source_validation.reference_outputs is not allowed; "
            "expected values belong in top-level expected_outputs"
        )
        raise ValueError(msg)
    return True


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

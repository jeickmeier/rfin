"""Executable helpers for integration golden fixtures."""

from __future__ import annotations

from tests.golden.conftest import _load_runner, fixture_path
from tests.golden.runners.calibration_common import (
    run_curve_fixture,
    run_hazard_fixture,
    run_sabr_cube_fixture,
)
from tests.golden.schema import GoldenFixture


def run_rates_integration(fixture: GoldenFixture) -> dict[str, float]:
    actuals: dict[str, float] = {}
    if calibration := fixture.inputs.get("calibration"):
        actuals.update(
            run_curve_fixture(_nested_fixture(fixture, calibration, _source_validation_outputs(calibration)))
        )
    if sabr := fixture.inputs.get("sabr_calibration"):
        actuals.update(run_sabr_cube_fixture(_nested_fixture(fixture, sabr, _source_validation_outputs(sabr))))
    actuals.update(_run_pricing(fixture))
    return actuals


def run_credit_integration(fixture: GoldenFixture) -> dict[str, float]:
    actuals: dict[str, float] = {}
    if hazard := fixture.inputs.get("hazard_calibration"):
        actuals.update(run_hazard_fixture(_nested_fixture(fixture, hazard, _source_validation_outputs(hazard))))
    actuals.update(_run_pricing(fixture))
    return actuals


def _run_pricing(fixture: GoldenFixture) -> dict[str, float]:
    if "pricing_fixture" in fixture.inputs:
        nested = GoldenFixture.from_path(fixture_path(fixture.inputs["pricing_fixture"]))
    else:
        nested = _nested_fixture(fixture, fixture.inputs["pricing"], {})
    runner = _load_runner(nested.domain)
    actuals = runner.run(nested)
    return {output: actuals[metric] for metric, output in fixture.inputs["pricing_metrics"].items()}


def _nested_fixture(fixture: GoldenFixture, inputs: dict, expected_outputs: dict[str, float]) -> GoldenFixture:
    return GoldenFixture(
        schema_version=fixture.schema_version,
        name=fixture.name,
        domain=fixture.domain,
        description=fixture.description,
        provenance=fixture.provenance,
        inputs=inputs,
        expected_outputs=expected_outputs,
        tolerances={},
    )


def _source_validation_outputs(inputs: dict) -> dict[str, float]:
    source_validation = inputs.get("source_validation")
    if not isinstance(source_validation, dict):
        return {}
    references = source_validation.get("reference_outputs")
    if not isinstance(references, dict):
        return {}
    return {metric: float(value) for metric, value in references.items()}

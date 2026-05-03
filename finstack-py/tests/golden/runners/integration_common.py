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
        _reject_nested_source_validation("curve calibration runner", calibration)
        actuals.update(run_curve_fixture(_nested_fixture(fixture, calibration, {})))
    if sabr := fixture.inputs.get("sabr_calibration"):
        _reject_nested_source_validation("SABR calibration runner", sabr)
        actuals.update(run_sabr_cube_fixture(_nested_fixture(fixture, sabr, {})))
    actuals.update(_run_pricing(fixture))
    return actuals


def run_credit_integration(fixture: GoldenFixture) -> dict[str, float]:
    actuals: dict[str, float] = {}
    if hazard := fixture.inputs.get("hazard_calibration"):
        _reject_nested_source_validation("hazard calibration runner", hazard)
        actuals.update(run_hazard_fixture(_nested_fixture(fixture, hazard, {})))
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


def _reject_nested_source_validation(runner: str, inputs: dict) -> None:
    if "source_validation" in inputs:
        msg = f"{runner} requires executable inputs; nested source_validation metadata cannot provide actuals"
        raise ValueError(msg)

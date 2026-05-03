"""Smoke tests for golden pytest helpers."""

from __future__ import annotations

from copy import deepcopy

import pytest

from .conftest import (
    DATA_ROOTS,
    WORKSPACE_ROOT,
    discover_fixtures,
    fixture_path,
    non_compared_metric_reason,
    run_golden,
    validate_fixture,
)
from .runners import attribution_common
from .schema import GoldenFixture, Provenance, ToleranceEntry
from .tolerance import compare


def test_fixture_path_pricing() -> None:
    path = fixture_path("pricing/irs/foo.json")
    assert path.parts[-3:] == ("pricing", "irs", "foo.json")
    assert "valuations" in str(path)


def test_fixture_path_analytics() -> None:
    path = fixture_path("analytics/returns/foo.json")
    assert "analytics" in str(path)


def test_fixture_path_unknown_domain_raises() -> None:
    with pytest.raises(ValueError, match="known top-level domain"):
        fixture_path("bogus/foo.json")


def test_discover_fixtures_empty_dir() -> None:
    assert discover_fixtures("pricing/nonexistent") == []


def test_run_golden_writes_reference_rows_for_source_validation_fixture() -> None:
    report = WORKSPACE_ROOT / "target/golden-reports/golden-comparisons.csv"

    run_golden("attribution/brinson_hood_beebower.json")

    csv = report.read_text(encoding="utf-8")
    assert (
        "runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason"
        in csv
    )
    assert "python,attribution/brinson_hood_beebower.json,total_active," in csv
    assert "python,attribution/brinson_hood_beebower.json,allocation::energy," in csv
    assert ",true," in csv


def test_attribution_raw_looking_keys_do_not_bypass_execution_requirement() -> None:
    fixture = _fixture(
        inputs={
            "components": {"selection::tech": 0.01},
            "sums": {"total_active": ["selection::tech"]},
            "holdings": [],
        },
        expected_outputs={"total_active": 0.01},
    )

    with pytest.raises(ValueError, match="requires executable inputs"):
        attribution_common.run(fixture)


def test_source_validation_reference_values_must_match_expected_outputs() -> None:
    fixture = _fixture(
        inputs={
            "components": {"selection::tech": 0.01},
            "source_validation": {
                "status": "non_executable",
                "reason": "unit test",
                "reference_outputs": {"selection::tech": 0.02},
            },
        },
        expected_outputs={"selection::tech": 0.01},
    )

    with pytest.raises(ValueError, match="does not exactly match expected_outputs"):
        attribution_common.run(fixture)


def test_validate_fixture_source_validation_requires_reason() -> None:
    fixture = _fixture(
        inputs={
            "components": {"selection::tech": 0.01},
            "source_validation": {
                "status": "non_executable",
                "reference_outputs": {"selection::tech": 0.01},
            },
        },
        expected_outputs={"selection::tech": 0.01},
    )

    with pytest.raises(ValueError, match="must explain"):
        validate_fixture(WORKSPACE_ROOT / "dummy.json", fixture)


def test_validate_fixture_source_validation_rejects_actual_outputs() -> None:
    fixture = _fixture(
        inputs={
            "actual_outputs": {"selection::tech": 0.01},
            "components": {"selection::tech": 0.01},
            "source_validation": {
                "status": "non_executable",
                "reason": "unit test",
                "reference_outputs": {"selection::tech": 0.01},
            },
        },
        expected_outputs={"selection::tech": 0.01},
    )

    with pytest.raises(ValueError, match="actual_outputs"):
        validate_fixture(WORKSPACE_ROOT / "dummy.json", fixture)


def test_abs_or_rel_tolerances_allow_either_by_default() -> None:
    result = compare("npv", 1_000_000.5, 1_000_000.0, ToleranceEntry(abs=0.01, rel=1e-6))

    assert result.passed


def test_abs_or_rel_tolerance_does_not_require_explicit_reason() -> None:
    result = compare(
        "npv",
        1_000_000.5,
        1_000_000.0,
        ToleranceEntry(
            abs=0.01,
            rel=1e-6,
            tolerance_reason="abs-or-rel tolerance reflects vendor screen rounding",
        ),
    )

    assert result.passed


def test_required_pricing_risk_metric_cannot_be_non_compared() -> None:
    fixture = _fixture(
        inputs={
            "source_reference": {
                "non_compared_metrics": ["cs01"],
                "non_compared_metrics_reason": "unit test",
            }
        },
        expected_outputs={"cs01": 1.0, "dv01": 2.0},
        domain="credit.cds_tranche",
    )

    assert non_compared_metric_reason(fixture, "cs01") is None
    with pytest.raises(AssertionError, match="required executable pricing/risk metrics"):
        validate_fixture(WORKSPACE_ROOT / "dummy.json", fixture)


def test_source_validation_cannot_hide_required_pricing_risk_metric() -> None:
    fixture = _fixture(
        inputs={
            "source_validation": {
                "status": "non_executable",
                "reason": "unit test",
                "reference_outputs": {"cs01": 1.0, "dv01": 2.0},
            },
            "source_reference": {
                "non_compared_metrics": ["cs01"],
                "non_compared_metrics_reason": "unit test",
            },
        },
        expected_outputs={"cs01": 1.0, "dv01": 2.0},
        domain="credit.cds_tranche",
    )

    with pytest.raises(AssertionError, match="required executable pricing/risk metrics"):
        validate_fixture(WORKSPACE_ROOT / "dummy.json", fixture)


def test_source_reference_non_compared_metric_does_not_bypass_comparison() -> None:
    fixture = _fixture(
        inputs={
            "source_reference": {
                "non_compared_metrics": ["npv"],
                "non_compared_metrics_reason": "unit test",
            }
        },
        expected_outputs={"npv": 1.0, "dv01": 2.0},
        domain="rates.irs",
    )

    assert non_compared_metric_reason(fixture, "npv") is None


def test_pricing_validation_rejects_invalid_instrument_json() -> None:
    path, fixture = _deposit_fixture()
    fixture.inputs["instrument_json"] = {
        "schema": "finstack.instrument/1",
        "instrument": {
            "type": "deposit",
            "spec": {},
        },
    }

    with pytest.raises(AssertionError, match="instrument_json"):
        validate_fixture(path, fixture)


def test_pricing_validation_rejects_unknown_metric_name() -> None:
    path, fixture = _deposit_fixture()
    fixture.inputs["metrics"] = ["deposit_par_rate", "dv01x"]

    with pytest.raises(AssertionError, match="dv01x"):
        validate_fixture(path, fixture)


def test_pricing_validation_requires_expected_metrics_to_be_requested() -> None:
    path, fixture = _deposit_fixture()
    fixture.inputs["metrics"] = ["deposit_par_rate"]

    with pytest.raises(AssertionError, match="dv01"):
        validate_fixture(path, fixture)


def _fixture(
    inputs: dict,
    expected_outputs: dict[str, float],
    domain: str = "attribution.equity",
) -> GoldenFixture:
    return GoldenFixture(
        schema_version="finstack.golden/1",
        name="unit_fixture",
        domain=domain,
        description="Unit fixture",
        provenance=Provenance(
            as_of="2026-04-30",
            source="formula",
            source_detail="unit test",
            captured_by="test",
            captured_on="2026-04-30",
            last_reviewed_by="test",
            last_reviewed_on="2026-04-30",
            review_interval_months=6,
            regen_command="",
        ),
        inputs=inputs,
        expected_outputs=expected_outputs,
        tolerances={metric: ToleranceEntry(abs=0.0) for metric in expected_outputs},
    )


def _deposit_fixture() -> tuple[object, GoldenFixture]:
    path = DATA_ROOTS["pricing"] / "pricing/deposit/usd_deposit_3m.json"
    fixture = GoldenFixture.from_path(path)
    fixture.inputs = deepcopy(fixture.inputs)
    return path, fixture

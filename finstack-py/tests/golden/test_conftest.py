"""Smoke tests for golden pytest helpers."""

from __future__ import annotations

import pytest

from .conftest import (
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


def test_run_golden_writes_comparison_csv() -> None:
    report = WORKSPACE_ROOT / "target/golden-reports/golden-comparisons.csv"

    run_golden("attribution/brinson_hood_beebower.json")

    csv = report.read_text(encoding="utf-8")
    assert (
        "runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason"
        in csv
    )
    assert "python,attribution/brinson_hood_beebower.json,__source_validation__" in csv
    assert ",true," in csv


def test_run_golden_writes_source_validation_status_row() -> None:
    report = WORKSPACE_ROOT / "target/golden-reports/golden-comparisons.csv"

    run_golden("attribution/brinson_hood_beebower.json")

    csv = report.read_text(encoding="utf-8")
    assert "python,attribution/brinson_hood_beebower.json,__source_validation__" in csv
    assert "source_validation: non_executable" in csv


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


def test_abs_and_rel_tolerances_require_both_by_default() -> None:
    result = compare("npv", 1_000_000.5, 1_000_000.0, ToleranceEntry(abs=0.01, rel=1e-6))

    assert not result.passed


def test_abs_or_rel_tolerance_requires_explicit_reason() -> None:
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

"""Smoke tests for golden pytest helpers."""

from __future__ import annotations

import pytest

from .conftest import WORKSPACE_ROOT, discover_fixtures, fixture_path, run_golden


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

    run_golden("pricing/irs/usd_sofr_5y_receive_fixed_swpm.json")

    csv = report.read_text(encoding="utf-8")
    assert (
        "runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason"
        in csv
    )
    assert "python,pricing/irs/usd_sofr_5y_receive_fixed_swpm.json,npv," in csv
    assert ",true," in csv

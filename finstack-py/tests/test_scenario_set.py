"""Tests for ScenarioSet functionality."""

from __future__ import annotations

from finstack.core.dates import PeriodId
import pytest

from finstack.statements import AmountOrScalar, ModelBuilder, ScenarioDefinition, ScenarioSet


def build_simple_model() -> tuple:
    """Build a simple financial model for testing."""
    period_q1 = PeriodId.quarter(2025, 1)
    period_q2 = PeriodId.quarter(2025, 2)

    builder = ModelBuilder.new("scenario_py")
    builder.periods("2025Q1..Q2", None)
    builder.value(
        "revenue",
        [
            (period_q1, AmountOrScalar.scalar(100_000.0)),
            (period_q2, AmountOrScalar.scalar(100_000.0)),
        ],
    )
    builder.compute("cogs", "revenue * 0.4")
    builder.compute("ebitda", "revenue - cogs")
    model = builder.build()

    return model, period_q1


def test_scenario_set_evaluate_and_diff() -> None:
    """Test evaluating scenarios and computing differences."""
    model, period = build_simple_model()

    # Build scenarios via Python API
    base_def = ScenarioDefinition(model_id=model.id)
    downside_def = ScenarioDefinition(
        parent="base",
        overrides={"revenue": 90_000.0},
        model_id=model.id,
    )

    scenarios = ScenarioSet()
    scenarios.add_scenario("base", base_def)
    scenarios.add_scenario("downside", downside_def)

    results = scenarios.evaluate_all(model)
    assert len(results) == 2

    base_results = results.get("base")
    downside_results = results.get("downside")

    assert base_results.get("revenue", period) == pytest.approx(100_000.0)
    assert downside_results.get("revenue", period) == pytest.approx(90_000.0)

    # Diff between base and downside
    metrics = ["revenue", "ebitda"]
    periods = [period]
    diff = scenarios.diff(results, "base", "downside", metrics, periods)

    assert diff.baseline == "base"
    assert diff.comparison == "downside"

    report = diff.variance
    rows = {row.metric(): row for row in report.rows()}

    assert rows["revenue"].baseline() == pytest.approx(100_000.0)
    assert rows["revenue"].comparison() == pytest.approx(90_000.0)

    assert rows["ebitda"].baseline() == pytest.approx(60_000.0)
    assert rows["ebitda"].comparison() == pytest.approx(54_000.0)


def test_scenario_set_from_mapping() -> None:
    """Test creating ScenarioSet from a mapping dictionary."""
    model, period = build_simple_model()

    mapping: dict[str, dict[str, object]] = {
        "base": {"model_id": model.id, "overrides": {}},
        "downside": {
            "parent": "base",
            "model_id": model.id,
            "overrides": {"revenue": 90_000.0},
        },
    }

    scenarios = ScenarioSet.from_mapping(mapping)
    assert set(scenarios.scenario_names) == {"base", "downside"}

    results = scenarios.evaluate_all(model)
    base_results = results.get("base")
    downside_results = results.get("downside")

    assert base_results.get("revenue", period) == pytest.approx(100_000.0)
    assert downside_results.get("revenue", period) == pytest.approx(90_000.0)

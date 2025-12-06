"""Tests for Monte Carlo evaluation of statement models."""

from __future__ import annotations

import pytest

from finstack.core.dates.periods import PeriodId
from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator, MonteCarloResults
from finstack.statements.types import AmountOrScalar, FinancialModelSpec, ForecastSpec


def build_simple_normal_model() -> FinancialModelSpec:
    """Build a simple model with normal forecast for testing."""
    builder = ModelBuilder.new("mc-test")
    builder.periods("2025Q1..Q4", "2025Q2")

    mixed = builder.mixed("revenue")
    mixed.values([
        (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100_000.0)),
        (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110_000.0)),
    ])
    mixed.forecast(ForecastSpec.normal(mean=120_000.0, std=10_000.0, seed=42))
    builder = mixed.finish()

    return builder.build()


def test_evaluate_monte_carlo_basic() -> None:
    """Test basic Monte Carlo evaluation functionality."""
    model = build_simple_normal_model()
    evaluator = Evaluator.new()

    mc: MonteCarloResults = evaluator.evaluate_monte_carlo(model, n_paths=32, seed=7, percentiles=[0.05, 0.5, 0.95])

    assert mc.n_paths == 32
    assert mc.percentiles == pytest.approx([0.05, 0.5, 0.95])

    # Should have a P95 series for revenue
    p95 = mc.get_percentile("revenue", 0.95)
    assert p95 is not None
    assert len(p95) > 0

    # Breach probability should be between 0 and 1
    prob = mc.breach_probability("revenue", threshold=100_000.0)
    assert prob is not None
    assert 0.0 <= prob <= 1.0


def test_evaluate_monte_carlo_deterministic() -> None:
    """Test that Monte Carlo evaluation produces deterministic results."""
    model = build_simple_normal_model()
    evaluator1 = Evaluator.new()
    evaluator2 = Evaluator.new()

    cfg = {"n_paths": 16, "seed": 123, "percentiles": [0.05, 0.5, 0.95]}

    mc1 = evaluator1.evaluate_monte_carlo(model, **cfg)
    mc2 = evaluator2.evaluate_monte_carlo(model, **cfg)

    assert mc1.n_paths == mc2.n_paths
    assert mc1.percentiles == pytest.approx(mc2.percentiles)

    p95_1 = mc1.get_percentile("revenue", 0.95)
    p95_2 = mc2.get_percentile("revenue", 0.95)
    assert p95_1 is not None
    assert p95_2 is not None
    # PeriodId objects are not equal across calls, so compare by string code.
    codes1 = {str(period): value for period, value in p95_1.items()}
    codes2 = {str(period): value for period, value in p95_2.items()}
    assert codes1.keys() == codes2.keys()
    for code, value in codes1.items():
        assert value == pytest.approx(codes2[code])

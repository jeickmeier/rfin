"""Tests for financial modeling patterns via Python bindings."""

from finstack.core.dates import PeriodId
from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator
import pytest


def test_roll_forward_pattern_python() -> None:
    """Test the roll-forward pattern via Python bindings."""
    # Build model with roll-forward pattern
    builder = ModelBuilder.new("SaaS Model")
    builder.periods("2025Q1..2025Q4", None)

    # Inputs
    builder.value_scalar(
        "new_arr",
        [
            (PeriodId.quarter(2025, 1), 100.0),
            (PeriodId.quarter(2025, 2), 120.0),
            (PeriodId.quarter(2025, 3), 140.0),
            (PeriodId.quarter(2025, 4), 160.0),
        ],
    )

    builder.value_scalar(
        "churn_arr",
        [
            (PeriodId.quarter(2025, 1), 10.0),
            (PeriodId.quarter(2025, 2), 12.0),
            (PeriodId.quarter(2025, 3), 14.0),
            (PeriodId.quarter(2025, 4), 16.0),
        ],
    )

    # Apply Pattern: End = Beg + New - Churn
    builder.add_roll_forward("arr", ["new_arr"], ["churn_arr"])

    model = builder.build()
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    # Verify Q1: Beg=0, End=0+100-10=90
    assert results.get("arr_beg", PeriodId.quarter(2025, 1)) == pytest.approx(0.0)
    assert results.get("arr_end", PeriodId.quarter(2025, 1)) == pytest.approx(90.0)

    # Verify Q2: Beg=90, End=90+120-12=198
    assert results.get("arr_beg", PeriodId.quarter(2025, 2)) == pytest.approx(90.0)
    assert results.get("arr_end", PeriodId.quarter(2025, 2)) == pytest.approx(198.0)

    # Verify Q3: Beg=198, End=198+140-14=324
    assert results.get("arr_beg", PeriodId.quarter(2025, 3)) == pytest.approx(198.0)
    assert results.get("arr_end", PeriodId.quarter(2025, 3)) == pytest.approx(324.0)


def test_vintage_buildup_pattern_python() -> None:
    """Test the vintage/cohort pattern via Python bindings."""
    # Decay curve: 100%, 80%, 50%, 0%
    decay_curve = [1.0, 0.8, 0.5, 0.0]

    builder = ModelBuilder.new("Cohort Model")
    builder.periods("2025Q1..2025Q4", None)

    builder.value_scalar(
        "new_sales",
        [
            (PeriodId.quarter(2025, 1), 100.0),
            (PeriodId.quarter(2025, 2), 200.0),
            (PeriodId.quarter(2025, 3), 300.0),
            (PeriodId.quarter(2025, 4), 400.0),
        ],
    )

    builder.add_vintage_buildup("revenue", "new_sales", decay_curve)

    model = builder.build()
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)

    # Q1: New=100 -> 100*1.0 = 100
    assert abs(results.get("revenue", PeriodId.quarter(2025, 1)) - 100.0) < 1e-6

    # Q2: New=200 (1.0) + Old=100 (0.8) = 200 + 80 = 280
    assert abs(results.get("revenue", PeriodId.quarter(2025, 2)) - 280.0) < 1e-6

    # Q3: New=300 (1.0) + Old=200 (0.8) + Oldest=100 (0.5) = 300 + 160 + 50 = 510
    assert abs(results.get("revenue", PeriodId.quarter(2025, 3)) - 510.0) < 1e-6

    # Q4: New=400 (1.0) + Old=300 (0.8) + Older=200 (0.5) + Oldest=100 (0.0) = 400 + 240 + 100 + 0 = 740
    assert abs(results.get("revenue", PeriodId.quarter(2025, 4)) - 740.0) < 1e-6

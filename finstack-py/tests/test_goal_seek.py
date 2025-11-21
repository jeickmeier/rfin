"""Test goal seek functionality in Python bindings."""

import pytest

from finstack.core.dates.periods import PeriodId
from finstack.statements.builder import ModelBuilder
from finstack.statements.evaluator import Evaluator
from finstack.statements.types import ForecastSpec


def test_goal_seek_simple_linear() -> None:
    """Test goal seek with simple linear relationship."""
    # Build a simple model: net_income = revenue * margin
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q1", None)

    period = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(period, 100_000.0)])
    builder.compute("profit_margin", "0.15")
    builder.compute("net_income", "revenue * profit_margin")

    model = builder.build()

    # Solve for revenue that gives $20,000 net income
    # Expected: 20,000 / 0.15 = 133,333.33
    solved = model.goal_seek(
        target_node="net_income",
        target_period="2025Q1",
        target_value=20_000.0,
        driver_node="revenue",
        update_model=False,
    )

    assert abs(solved - 133_333.33) < 1.0


@pytest.mark.xfail(reason="Requires better initial guess when value only exists in different period")
def test_goal_seek_with_update() -> None:
    """Test goal seek with model update."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q1", None)

    period = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(period, 100_000.0)])
    builder.compute("cogs", "revenue * 0.6")
    builder.compute("gross_profit", "revenue - cogs")

    model = builder.build()

    # Solve for revenue that gives $50,000 gross profit
    # Expected: 50,000 / 0.4 = 125,000
    solved = model.goal_seek(
        target_node="gross_profit",
        target_period="2025Q1",
        target_value=50_000.0,
        driver_node="revenue",
        update_model=True,
    )

    assert abs(solved - 125_000.0) < 1.0

    # Verify the model was updated
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)
    revenue = results.get("revenue", "2025Q1")
    assert abs(revenue - 125_000.0) < 1.0


@pytest.mark.xfail(reason="Solver struggles with forecast-driven dependencies across periods")
def test_goal_seek_interest_coverage() -> None:
    """Test realistic case: solve for revenue to achieve target interest coverage."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q4", None)

    q1 = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(q1, 100_000.0)])
    builder.forecast("revenue", ForecastSpec.growth(0.05))
    builder.compute("interest_expense", "10000.0")
    builder.compute("ebitda", "revenue * 0.3")
    builder.compute("interest_coverage", "ebitda / interest_expense")

    model = builder.build()

    # Solve for Q4 revenue that achieves 2.0x interest coverage
    # Expected: interest_coverage = (revenue * 0.3) / 10000 = 2.0
    # So revenue = 2.0 * 10000 / 0.3 = 66,666.67
    solved = model.goal_seek(
        target_node="interest_coverage",
        target_period="2025Q4",
        target_value=2.0,
        driver_node="revenue",
        driver_period="2025Q4",
        update_model=True,
    )

    assert abs(solved - 66_666.67) < 1.0

    # Verify the solution by evaluating
    evaluator = Evaluator.new()
    results = evaluator.evaluate(model)
    coverage = results.get("interest_coverage", "2025Q4")
    assert abs(coverage - 2.0) < 0.01


@pytest.mark.xfail(reason="Requires better initial guess when driver period differs from value period")
def test_goal_seek_default_driver_period() -> None:
    """Test that driver_period defaults to target_period."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q2", None)

    q1 = PeriodId.quarter(2025, 1)
    q2 = PeriodId.quarter(2025, 2)
    builder.value("revenue", [(q1, 100_000.0), (q2, 100_000.0)])
    builder.compute("profit_margin", "0.15")
    builder.compute("net_income", "revenue * profit_margin")

    model = builder.build()

    # Solve for Q2 revenue without specifying driver_period (should default to Q2)
    solved = model.goal_seek(
        target_node="net_income",
        target_period="2025Q2",
        target_value=20_000.0,
        driver_node="revenue",
        update_model=False,
    )

    assert abs(solved - 133_333.33) < 1.0


def test_goal_seek_invalid_target_node() -> None:
    """Test error handling for invalid target node."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q1", None)

    period = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(period, 100_000.0)])

    model = builder.build()

    with pytest.raises(ValueError, match="Target node"):
        model.goal_seek(
            target_node="nonexistent",
            target_period="2025Q1",
            target_value=1000.0,
            driver_node="revenue",
            update_model=False,
        )


def test_goal_seek_invalid_driver_node() -> None:
    """Test error handling for invalid driver node."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q1", None)

    period = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(period, 100_000.0)])

    model = builder.build()

    with pytest.raises(ValueError, match="Driver node"):
        model.goal_seek(
            target_node="revenue",
            target_period="2025Q1",
            target_value=1000.0,
            driver_node="nonexistent",
            update_model=False,
        )


def test_goal_seek_invalid_target_period() -> None:
    """Test error handling for invalid target period."""
    builder = ModelBuilder.new("test")
    builder.periods("2025Q1..Q1", None)

    period = PeriodId.quarter(2025, 1)
    builder.value("revenue", [(period, 100_000.0)])
    builder.compute("profit_margin", "0.15")
    builder.compute("net_income", "revenue * profit_margin")

    model = builder.build()

    with pytest.raises(ValueError, match="Invalid target period"):
        model.goal_seek(
            target_node="net_income",
            target_period="2025Q5",  # Invalid quarter
            target_value=20_000.0,
            driver_node="revenue",
            update_model=False,
        )


#!/usr/bin/env python
"""Simple test script for scenarios module."""
# ruff: noqa: T201

from datetime import date

import finstack

# Import types
Currency = finstack.Currency
MarketContext = finstack.market_data.MarketContext
DiscountCurve = finstack.market_data.DiscountCurve
MarketScalar = finstack.market_data.MarketScalar
Money = finstack.Money
FinancialModelSpec = finstack.statements.types.FinancialModelSpec

ScenarioSpec = finstack.scenarios.ScenarioSpec
OperationSpec = finstack.scenarios.OperationSpec
ScenarioEngine = finstack.scenarios.ScenarioEngine
ExecutionContext = finstack.scenarios.ExecutionContext
CurveKind = finstack.scenarios.CurveKind


def test_basic_enum() -> None:
    """Test basic enum functionality."""
    print("Testing enums...")
    assert CurveKind.Discount == CurveKind.Discount
    assert CurveKind.Discount != CurveKind.Forecast
    print("✓ Enum tests passed")


def test_operation_creation() -> None:
    """Test creating operations."""
    print("Testing operation creation...")
    OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0)
    OperationSpec.equity_price_pct(["SPY"], -10.0)
    OperationSpec.stmt_forecast_percent("Revenue", -5.0)
    print("✓ Operation creation tests passed")


def test_scenario_creation() -> None:
    """Test creating scenario."""
    print("Testing scenario creation...")
    ops = [
        OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0),
        OperationSpec.equity_price_pct(["SPY"], -10.0),
    ]
    scenario = ScenarioSpec("test_scenario", ops, name="Test Scenario", priority=0)
    assert scenario.id == "test_scenario"
    assert scenario.name == "Test Scenario"
    assert len(scenario.operations) == 2
    print("✓ Scenario creation tests passed")


def test_engine_creation() -> None:
    """Test creating engine."""
    print("Testing engine creation...")
    engine = ScenarioEngine()
    assert engine is not None
    print("✓ Engine creation tests passed")


def test_context_creation() -> None:
    """Test creating execution context."""
    print("Testing execution context...")
    market = MarketContext()
    model = FinancialModelSpec("test", [])
    as_of = date(2025, 1, 1)
    ctx = ExecutionContext(market, model, as_of)
    assert ctx.as_of == as_of
    print("✓ Context creation tests passed")


def test_apply_empty_scenario() -> None:
    """Test applying empty scenario."""
    print("Testing empty scenario application...")
    market = MarketContext()
    model = FinancialModelSpec("test", [])
    as_of = date(2025, 1, 1)

    scenario = ScenarioSpec("empty", [])
    engine = ScenarioEngine()
    ctx = ExecutionContext(market, model, as_of)
    report = engine.apply(scenario, ctx)

    assert report.operations_applied == 0
    print("✓ Empty scenario application test passed")


def test_curve_shock() -> None:
    """Test curve shock application."""
    print("Testing curve shock...")
    base_date = date(2025, 1, 1)

    # Build discount curve using simple constructor
    knots = [(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)]
    curve = DiscountCurve("USD-OIS", base_date, knots)

    market = MarketContext()
    market.insert_discount(curve)
    model = FinancialModelSpec("test", [])

    # Create and apply scenario
    scenario = ScenarioSpec("rate_shock", [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0)])

    engine = ScenarioEngine()
    ctx = ExecutionContext(market, model, base_date)
    report = engine.apply(scenario, ctx)

    assert report.operations_applied == 1

    # Verify curve was shocked
    shocked_curve = market.discount("USD-OIS")
    df_1y = shocked_curve.df(1.0)
    assert df_1y < 0.98, f"Expected df < 0.98, got {df_1y}"
    print("✓ Curve shock test passed")


def test_serde() -> None:
    """Test JSON serialization."""
    print("Testing JSON serialization...")
    ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0)]
    scenario = ScenarioSpec("test", ops)

    # Test to_dict/from_dict
    data = scenario.to_dict()
    assert isinstance(data, dict)

    scenario2 = ScenarioSpec.from_dict(data)
    assert scenario2.id == scenario.id

    # Test to_json/from_json
    json_str = scenario.to_json()
    assert isinstance(json_str, str)

    scenario3 = ScenarioSpec.from_json(json_str)
    assert scenario3.id == scenario.id
    print("✓ Serialization tests passed")


if __name__ == "__main__":
    test_basic_enum()
    test_operation_creation()
    test_scenario_creation()
    test_engine_creation()
    test_context_creation()
    test_apply_empty_scenario()
    test_curve_shock()
    test_serde()
    print("\n✅ All tests passed!")

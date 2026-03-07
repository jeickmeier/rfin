#!/usr/bin/env python
"""Example demonstrating the finstack.scenarios Python bindings.

This example shows how to:
1. Create scenario operations for market shocks
2. Compose scenarios with priorities
3. Apply scenarios to market and statement contexts
4. Extract results and analyze impacts
"""

from datetime import date

import finstack

# Import required types
Currency = finstack.Currency
MarketContext = finstack.market_data.context.MarketContext
DiscountCurve = finstack.market_data.term_structures.DiscountCurve
MarketScalar = finstack.market_data.scalars.MarketScalar
Money = finstack.Money
FinancialModelSpec = finstack.statements.types.FinancialModelSpec

# Scenarios imports
ScenarioSpec = finstack.scenarios.ScenarioSpec
OperationSpec = finstack.scenarios.OperationSpec
ScenarioEngine = finstack.scenarios.ScenarioEngine
ExecutionContext = finstack.scenarios.ExecutionContext
CurveKind = finstack.scenarios.CurveKind
VolSurfaceKind = finstack.scenarios.VolSurfaceKind
TenorMatchMode = finstack.scenarios.TenorMatchMode


def main() -> None:
    """Run scenarios example."""
    print("=" * 70)
    print("Finstack Scenarios Python Bindings Example")
    print("=" * 70)

    # 1. Setup market data
    print("\n1. Setting up market data...")
    base_date = date(2025, 1, 1)

    # Create discount curve
    knots = [(0.0, 1.0), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)]
    usd_curve = DiscountCurve("USD-OIS", base_date, knots)

    market = MarketContext()
    market.insert(usd_curve)

    # Add equity prices
    market.insert_price("SPY", MarketScalar.get_price(Money(450.0, Currency("USD"))))
    market.insert_price("QQQ", MarketScalar.get_price(Money(380.0, Currency("USD"))))

    print("  - Created USD discount curve")
    print("  - Added SPY price: $450")
    print("  - Added QQQ price: $380")

    # 2. Create financial model
    print("\n2. Creating financial model...")
    model = FinancialModelSpec("demo_model", [])
    print("  - Created empty financial model")

    # 3. Define scenarios
    print("\n3. Defining scenarios...")

    # Base rate shock scenario
    rate_shock = ScenarioSpec(
        "rate_shock",
        [
            OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0),
        ],
        name="Rate Shock +50bp",
        description="Parallel shift of USD discount curve",
        priority=0,
    )
    print("  - Created rate shock scenario (+50bp)")

    # Equity crash scenario
    equity_crash = ScenarioSpec(
        "equity_crash",
        [
            OperationSpec.equity_price_pct(["SPY", "QQQ"], -20.0),
        ],
        name="Equity Crash -20%",
        description="Market sell-off scenario",
        priority=1,
    )
    print("  - Created equity crash scenario (-20%)")

    # Combined stress scenario
    combined_stress = ScenarioSpec(
        "combined_stress",
        [
            OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 100.0),
            OperationSpec.equity_price_pct(["SPY", "QQQ"], -30.0),
        ],
        name="Combined Stress",
        description="Rates up 100bp + Equities down 30%",
        priority=2,
    )
    print("  - Created combined stress scenario")

    # 4. Compose scenarios
    print("\n4. Testing scenario composition...")
    engine = ScenarioEngine()
    composed = engine.compose([rate_shock, equity_crash, combined_stress])
    print(f"  - Composed {len(composed.operations)} operations from 3 scenarios")
    print(f"  - Composed scenario ID: {composed.id}")

    # 5. Apply rate shock scenario
    print("\n5. Applying rate shock scenario...")
    ctx = ExecutionContext(market, model, base_date)
    report = engine.apply(rate_shock, ctx)

    print(f"  - Operations applied: {report.operations_applied}")
    print(f"  - Warnings: {len(report.warnings)}")

    # Check shocked curve
    shocked_curve = market.get_discount("USD-OIS")
    original_df = 0.98
    shocked_df = shocked_curve.df(1.0)
    print(f"  - Original 1Y DF: {original_df:.6f}")
    print(f"  - Shocked 1Y DF: {shocked_df:.6f}")
    print(f"  - Change: {(shocked_df - original_df) * 10000:.2f} bps")

    # 6. Test equity shock
    print("\n6. Applying equity crash scenario...")

    # Reset market for clean test
    market2 = MarketContext()
    market2.insert_price("SPY", MarketScalar.get_price(Money(450.0, Currency("USD"))))

    ctx2 = ExecutionContext(market2, model, base_date)
    report2 = engine.apply(equity_crash, ctx2)

    print(f"  - Operations applied: {report2.operations_applied}")

    shocked_spy = market2.get_price("SPY")
    print("  - Original SPY: $450.00")
    print(f"  - Shocked SPY: ${shocked_spy.value.amount:.2f}")

    # 7. Test JSON serialization
    print("\n7. Testing JSON serialization...")
    json_str = rate_shock.to_json()
    print(f"  - Serialized to JSON ({len(json_str)} chars)")

    rehydrated = ScenarioSpec.from_json(json_str)
    print(f"  - Deserialized scenario ID: {rehydrated.id}")
    print(f"  - Deserialized operations: {len(rehydrated.operations)}")

    # 8. Test node-specific curve shocks
    print("\n8. Testing tenor-specific curve shocks...")
    node_shock = ScenarioSpec(
        "steepener",
        [
            OperationSpec.curve_node_bp(
                CurveKind.Discount, "USD-OIS", [("5Y", 25.0), ("10Y", 50.0)], TenorMatchMode.Interpolate
            ),
        ],
        name="Curve Steepener",
    )

    # Create fresh market
    market3 = MarketContext()
    market3.insert(DiscountCurve("USD-OIS", base_date, knots))
    ctx3 = ExecutionContext(market3, model, base_date)

    report3 = engine.apply(node_shock, ctx3)
    print("  - Applied tenor-specific shock")
    print(f"  - Operations applied: {report3.operations_applied}")

    print("\n" + "=" * 70)
    print("✅ Example completed successfully!")
    print("=" * 70)


if __name__ == "__main__":
    main()

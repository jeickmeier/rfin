#!/usr/bin/env python
"""Example demonstrating the finstack.io Python bindings.

This example shows how to:
1. Open/create a SQLite database for persistence
2. Save and load market data (curves, contexts)
3. Use bulk operations for efficiency
4. Use lookback queries for historical data
5. Save and load instruments
6. Save and load portfolio specifications (with instrument references)
7. Save and load metric registries
8. Save and load statement models (financial modeling)
9. Save and load scenarios

Run with: uv run examples/scripts/io/io_persistence_example.py

The database is created at: examples/scripts/io/finstack_example.db
"""

from __future__ import annotations

from datetime import date
import os
from pathlib import Path

import finstack

# Core imports
Currency = finstack.Currency
Money = finstack.Money
MarketContext = finstack.core.market_data.context.MarketContext
DiscountCurve = finstack.core.market_data.term_structures.DiscountCurve
MarketScalar = finstack.core.market_data.scalars.MarketScalar

# IO imports
SqliteStore = finstack.io.SqliteStore
PortfolioSpec = finstack.io.PortfolioSpec
MarketContextSnapshot = finstack.io.MarketContextSnapshot
PortfolioSnapshot = finstack.io.PortfolioSnapshot

# Statements imports (for metric registry)
MetricRegistry = finstack.statements.registry.MetricRegistry
MetricDefinition = finstack.statements.registry.MetricDefinition
UnitType = finstack.statements.registry.UnitType

# Scenarios imports
ScenarioSpec = finstack.scenarios.ScenarioSpec
OperationSpec = finstack.scenarios.OperationSpec
CurveKind = finstack.scenarios.CurveKind


def main() -> None:
    """Run the IO persistence example."""
    print("=" * 70)
    print("Finstack IO - Python Persistence Example")
    print("=" * 70)

    backend = os.getenv("FINSTACK_IO_BACKEND", "sqlite").strip().lower()

    if backend in {"postgres", "postgresql"}:
        postgres_url = os.getenv("FINSTACK_POSTGRES_URL")
        if not postgres_url:
            raise RuntimeError("FINSTACK_POSTGRES_URL is required for postgres backend")
        PostgresStore = getattr(finstack.io, "PostgresStore", None)
        if PostgresStore is None:
            raise RuntimeError("PostgresStore is not available in this build")
        store = PostgresStore.connect(postgres_url)
        print("✅ Postgres database opened successfully")
        print(f"   URL: {postgres_url}\n")
    else:
        # Create a persistent database in the same directory as this script
        script_dir = Path(__file__).parent
        db_path = os.getenv("FINSTACK_SQLITE_PATH")
        if db_path:
            db_path = Path(db_path)
        else:
            db_path = script_dir / "finstack_example.db"
        print(f"\n📁 Database path: {db_path}\n")

        # Open (or create) the database - migrations run automatically
        store = SqliteStore.open(str(db_path))
        print("✅ Database opened successfully")
        print(f"   Path: {store.path}\n")

    # =========================================================================
    # 1. SAVING AND LOADING MARKET DATA
    # =========================================================================
    print("📈 1. Market Data Persistence")
    print("   " + "-" * 30)

    as_of = date(2024, 1, 15)

    # Create a discount curve with knot points
    usd_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),  # Today
            (0.25, 0.9875),  # 3M
            (0.5, 0.975),  # 6M
            (1.0, 0.95),  # 1Y
            (2.0, 0.90),  # 2Y
            (5.0, 0.80),  # 5Y
            (10.0, 0.67),  # 10Y
        ],
    )

    # Create a market context with the curve
    market_ctx = MarketContext()
    market_ctx.insert_discount(usd_curve)

    # Add some equity prices
    market_ctx.insert_price("SPY", MarketScalar.price(Money(450.0, Currency("USD"))))
    market_ctx.insert_price("QQQ", MarketScalar.price(Money(380.0, Currency("USD"))))

    # Save with optional metadata for provenance tracking
    meta = {"source": "example", "curve_build_version": "1.0"}
    store.put_market_context("DEFAULT", as_of, market_ctx, meta)
    print(f"   ✅ Saved market context for {as_of}")

    # Load it back
    loaded_ctx = store.get_market_context("DEFAULT", as_of)
    if loaded_ctx is not None:
        print("   ✅ Loaded market context")
        loaded_curve = loaded_ctx.discount("USD-OIS")
        print(f"      Curve ID: {loaded_curve.id}")
        print(f"      1Y DF: {loaded_curve.df(1.0):.6f}")
        print(f"      5Y DF: {loaded_curve.df(5.0):.6f}")

    # =========================================================================
    # 2. BULK OPERATIONS (More Efficient)
    # =========================================================================
    print("\n⚡ 2. Bulk Operations")
    print("   " + "-" * 30)

    # Create multiple market contexts for different dates
    dates_to_save = [
        date(2024, 1, 16),
        date(2024, 1, 17),
        date(2024, 1, 18),
        date(2024, 1, 19),
    ]

    batch = []
    for d in dates_to_save:
        # Create a slightly different curve for each day
        day_offset = (d - as_of).days
        curve = DiscountCurve(
            "USD-OIS",
            d,
            [
                (0.0, 1.0),
                (1.0, 0.95 + day_offset * 0.001),  # Slightly different 1Y DF
                (5.0, 0.80 + day_offset * 0.002),  # Slightly different 5Y DF
            ],
        )
        ctx = MarketContext()
        ctx.insert_discount(curve)
        batch.append(("DEFAULT", d, ctx))

    store.put_market_contexts_batch(batch)
    print(f"   ✅ Bulk saved {len(batch)} market contexts in single transaction")

    # =========================================================================
    # 3. LOOKBACK QUERIES (Historical Data)
    # =========================================================================
    print("\n📊 3. Lookback Queries")
    print("   " + "-" * 30)

    # List all market contexts in a date range
    start_date = date(2024, 1, 15)
    end_date = date(2024, 1, 19)
    snapshots = store.list_market_contexts("DEFAULT", start_date, end_date)

    print(f"   Found {len(snapshots)} market context snapshots:")
    for snap in snapshots:
        curve = snap.context.discount("USD-OIS")
        print(f"      {snap.as_of} -> 1Y DF: {curve.df(1.0):.6f}")

    # Get the latest context on or before a specific date
    query_date = date(2024, 1, 17)
    latest = store.latest_market_context_on_or_before("DEFAULT", query_date)
    if latest:
        print(f"   Latest context on or before {query_date}: {latest.as_of}")

    # =========================================================================
    # 4. INSTRUMENTS (Store Before Portfolios Reference Them)
    # =========================================================================
    print("\n📋 4. Instruments")
    print("   " + "-" * 30)

    # Instruments are stored as tagged union JSON: {"type": "...", "spec": {...}}
    # We store these BEFORE portfolios so portfolios can reference them by ID.

    # Store equity instruments for our portfolio positions
    # Instruments use tagged union JSON: {"type": "equity", "spec": {...}}
    # The "attributes" field requires {"tags": [], "meta": {}} structure
    spy_equity = {
        "type": "equity",
        "spec": {
            "id": "EQUITY_SPY",
            "ticker": "SPY",
            "currency": "USD",
            "shares": 1.0,
            "discount_curve_id": "USD-OIS",
            "attributes": {"tags": [], "meta": {}},
        },
    }
    store.put_instrument("EQUITY_SPY", spy_equity)
    print("   ✅ Saved EQUITY_SPY instrument")

    qqq_equity = {
        "type": "equity",
        "spec": {
            "id": "EQUITY_QQQ",
            "ticker": "QQQ",
            "currency": "USD",
            "shares": 1.0,
            "discount_curve_id": "USD-OIS",
            "attributes": {"tags": [], "meta": {}},
        },
    }
    store.put_instrument("EQUITY_QQQ", qqq_equity)
    print("   ✅ Saved EQUITY_QQQ instrument")

    # Bulk instrument save
    iwm_equity = {
        "type": "equity",
        "spec": {
            "id": "EQUITY_IWM",
            "ticker": "IWM",
            "currency": "USD",
            "shares": 1.0,
            "discount_curve_id": "USD-OIS",
            "attributes": {"tags": ["small_cap"], "meta": {"sector": "broad_market"}},
        },
    }
    store.put_instruments_batch([("EQUITY_IWM", iwm_equity)])
    print("   ✅ Bulk saved 1 instrument")

    # Load an instrument back
    loaded_instr = store.get_instrument("EQUITY_SPY")
    if loaded_instr:
        print(f"   ✅ Loaded instrument: type={loaded_instr['type']}")
        print(f"      ID: {loaded_instr['spec']['id']}")
        print(f"      Ticker: {loaded_instr['spec']['ticker']}")

    # =========================================================================
    # 5. PORTFOLIO SPECIFICATIONS
    # =========================================================================
    print("\n💼 5. Portfolio Specifications")
    print("   " + "-" * 30)

    # Create a portfolio spec as a dict (JSON-serializable)
    # Note: PositionUnit is an enum with snake_case variants:
    #   - "units" for shares/units
    #   - "face_value" for bonds/debt
    #   - "percentage" for percentage ownership
    #   - {"notional": "USD"} or {"notional": null} for notional amount
    portfolio_spec = {
        "id": "TREASURY_DESK",
        "name": "Treasury Trading Desk",
        "base_ccy": "USD",
        "as_of": "2024-01-15",
        "entities": {
            "FUND_A": {
                "id": "FUND_A",
                "name": "Alpha Fund",
                "tags": {"strategy": "macro"},
                "meta": {},
            },
            "FUND_B": {
                "id": "FUND_B",
                "name": "Beta Fund",
                "tags": {"strategy": "relative_value"},
                "meta": {},
            },
        },
        "positions": [
            {
                "position_id": "POS_001",
                "entity_id": "FUND_A",
                "instrument_id": "EQUITY_SPY",  # References instrument stored above
                "quantity": 1000.0,
                "unit": "units",  # Number of shares
                "tags": {"book": "equities"},
                "meta": {},
            },
            {
                "position_id": "POS_002",
                "entity_id": "FUND_B",
                "instrument_id": "EQUITY_QQQ",  # References instrument stored above
                "quantity": 500.0,
                "unit": "units",  # Number of shares
                "tags": {"book": "equities"},
                "meta": {},
            },
        ],
        "books": {},
        "tags": {"desk": "treasury"},
        "meta": {"last_reconciled": "2024-01-14"},
    }

    store.put_portfolio_spec("TREASURY_DESK", as_of, portfolio_spec)
    print(f"   ✅ Saved portfolio spec for {as_of}")

    # Load and inspect the portfolio spec
    spec = store.get_portfolio_spec("TREASURY_DESK", as_of)
    if spec is not None:
        print("   ✅ Loaded portfolio spec")
        print(f"      ID: {spec.id}")
        print(f"      Name: {spec.name}")
        print(f"      Base CCY: {spec.base_ccy}")
        print(f"      As-of: {spec.as_of}")
        print(f"      Positions: {spec.position_count}")
        print(f"      Entities: {spec.entity_count}")

        # Convert to dict for inspection
        spec_dict = spec.to_dict()
        print(f"      Dict keys: {list(spec_dict.keys())}")

    # =========================================================================
    # 6. PORTFOLIO LOOKBACK
    # =========================================================================
    print("\n📆 6. Portfolio Lookback")
    print("   " + "-" * 30)

    # Save multiple portfolio snapshots
    for i, d in enumerate(dates_to_save):
        spec_copy = portfolio_spec.copy()
        spec_copy["as_of"] = d.isoformat()
        # Adjust position quantity slightly for each day
        spec_copy["positions"] = [
            {
                "position_id": "POS_001",
                "entity_id": "FUND_A",
                "instrument_id": "EQUITY_SPY",  # References stored instrument
                "quantity": 1000.0 + i * 100.0,
                "unit": "units",  # Number of shares
                "tags": {},
                "meta": {},
            }
        ]
        store.put_portfolio_spec("TREASURY_DESK", d, spec_copy)

    print(f"   ✅ Saved portfolio specs for {len(dates_to_save)} dates")

    # Query portfolio history
    port_snapshots = store.list_portfolios("TREASURY_DESK", date(2024, 1, 15), date(2024, 1, 19))
    print(f"   Found {len(port_snapshots)} portfolio snapshots:")
    for snap in port_snapshots:
        print(f"      {snap.as_of}: {snap.spec.position_count} positions")

    # Get latest portfolio
    latest_port = store.latest_portfolio_on_or_before("TREASURY_DESK", date(2024, 1, 17))
    if latest_port:
        print(f"   Latest portfolio on or before Jan 17: {latest_port.as_of}")

    # =========================================================================
    # 7. METRIC REGISTRIES
    # =========================================================================
    print("\n📐 7. Metric Registries")
    print("   " + "-" * 30)

    # Create a standard financial metrics registry
    # MetricDefinition(id, name, formula, description=, category=, unit_type=, requires=, tags=)
    # MetricRegistry(namespace, metrics, schema_version)
    fin_registry = MetricRegistry(
        "fin",
        [
            MetricDefinition(
                "gross_margin",
                "Gross Margin %",
                "gross_profit / revenue",
                description="Gross profit as percentage of revenue",
                category="margins",
                unit_type=UnitType.PERCENTAGE,
                requires=["revenue", "gross_profit"],
                tags=["margins", "profitability"],
            ),
            MetricDefinition(
                "ebitda_margin",
                "EBITDA Margin %",
                "ebitda / revenue",
                description="EBITDA as percentage of revenue",
                category="margins",
                unit_type=UnitType.PERCENTAGE,
                requires=["revenue", "ebitda"],
                tags=["margins", "profitability"],
            ),
            MetricDefinition(
                "debt_to_ebitda",
                "Debt to EBITDA",
                "total_debt / ebitda",
                description="Leverage ratio",
                category="leverage",
                unit_type=UnitType.RATIO,
                requires=["total_debt", "ebitda"],
                tags=["leverage", "credit"],
            ),
        ],
        1,  # schema_version
    )

    # Create a custom metrics registry
    custom_registry = MetricRegistry(
        "custom",
        [
            MetricDefinition(
                "revenue_growth",
                "Revenue Growth %",
                "(revenue - revenue[-1]) / revenue[-1]",
                description="Quarter-over-quarter revenue growth rate",
                category="growth",
                unit_type=UnitType.PERCENTAGE,
                requires=["revenue"],
                tags=["growth"],
            ),
        ],
        1,  # schema_version
    )

    # Store both registries
    store.put_metric_registry("fin", fin_registry)
    store.put_metric_registry("custom", custom_registry)
    print("   ✅ Saved metric registries: 'fin' and 'custom'")

    # List all available registries
    namespaces = store.list_metric_registries()
    print(f"   Available namespaces: {namespaces}")

    # Load and display each registry
    for ns in namespaces:
        loaded = store.load_metric_registry(ns)
        print(f"\n   📊 Registry '{loaded.namespace}' ({len(loaded.metrics)} metrics):")
        for metric in loaded.metrics:
            print(f"      {ns}.{metric.id} = {metric.formula}")
            if metric.description:
                print(f"         {metric.description}")

    # Demonstrate deletion
    print("\n   Deleting 'custom' registry...")
    deleted = store.delete_metric_registry("custom")
    print(f"   Deleted: {deleted}")

    remaining = store.list_metric_registries()
    print(f"   Remaining namespaces: {remaining}")

    # =========================================================================
    # 8. STATEMENT MODELS (Financial Model Persistence)
    # =========================================================================
    print("\n📊 8. Statement Models")
    print("   " + "-" * 30)

    # Import statement types
    from finstack.core.dates.periods import PeriodId, build_periods
    from finstack.statements.types import (
        AmountOrScalar,
        FinancialModelSpec,
        ForecastSpec,
        NodeSpec,
        NodeType,
    )

    # Build quarterly periods for 2024 (Q1-Q2 are actuals, Q3-Q4 are forecasts)
    period_plan = build_periods("2024Q1..Q4", "2024Q2")
    print(f"   Built {len(period_plan.periods)} periods:")
    for p in period_plan.periods:
        kind = "actual" if p.is_actual else "forecast"
        print(f"      {p.id} - {kind}")

    # Create a financial model specification
    model = FinancialModelSpec("quarterly_pnl", period_plan.periods)

    # Add revenue node with explicit values for actuals and growth forecast
    revenue = (
        NodeSpec("revenue", NodeType.MIXED)
        .with_name("Total Revenue")
        .with_values([
            (PeriodId.quarter(2024, 1), AmountOrScalar.amount(1_500_000.0, Currency("USD"))),
            (PeriodId.quarter(2024, 2), AmountOrScalar.amount(1_650_000.0, Currency("USD"))),
        ])
        .with_forecast(ForecastSpec.growth(0.10))  # 10% QoQ growth for forecasts
        .with_tags(["income_statement", "top_line"])
    )
    model.add_node(revenue)

    # Add COGS as a calculated node (formula-based)
    cogs = (
        NodeSpec("cogs", NodeType.CALCULATED)
        .with_name("Cost of Goods Sold")
        .with_formula("revenue * -0.60")  # 60% COGS margin
        .with_tags(["income_statement", "expense"])
    )
    model.add_node(cogs)

    # Add Gross Profit as calculated
    gross_profit = (
        NodeSpec("gross_profit", NodeType.CALCULATED)
        .with_name("Gross Profit")
        .with_formula("revenue + cogs")  # Revenue - COGS (COGS is negative)
        .with_tags(["income_statement", "subtotal"])
    )
    model.add_node(gross_profit)

    # Add Operating Expenses with curve-based forecast (different growth per period)
    opex = (
        NodeSpec("opex", NodeType.MIXED)
        .with_name("Operating Expenses")
        .with_values([
            (PeriodId.quarter(2024, 1), AmountOrScalar.amount(-350_000.0, Currency("USD"))),
            (PeriodId.quarter(2024, 2), AmountOrScalar.amount(-375_000.0, Currency("USD"))),
        ])
        .with_forecast(ForecastSpec.curve([0.05, 0.03]))  # Q3: +5%, Q4: +3%
        .with_tags(["income_statement", "expense"])
    )
    model.add_node(opex)

    # Add headcount with forward-fill forecast (carry last value)
    headcount = (
        NodeSpec("headcount", NodeType.MIXED)
        .with_name("Employee Headcount")
        .with_values([
            (PeriodId.quarter(2024, 1), AmountOrScalar.scalar(45.0)),
            (PeriodId.quarter(2024, 2), AmountOrScalar.scalar(48.0)),
        ])
        .with_forecast(ForecastSpec.forward_fill())  # Headcount stays flat
        .with_tags(["kpi", "operating"])
    )
    model.add_node(headcount)

    # Add EBITDA as calculated
    ebitda = (
        NodeSpec("ebitda", NodeType.CALCULATED)
        .with_name("EBITDA")
        .with_formula("gross_profit + opex")
        .with_tags(["income_statement", "profit"])
    )
    model.add_node(ebitda)

    # Add revenue per employee as a derived KPI
    rev_per_head = (
        NodeSpec("revenue_per_employee", NodeType.CALCULATED)
        .with_name("Revenue per Employee")
        .with_formula("revenue / headcount")
        .with_tags(["kpi", "efficiency"])
    )
    model.add_node(rev_per_head)

    # Store the model
    store.put_statement_model("QUARTERLY_PNL", model)
    print(f"\n   ✅ Saved statement model: {model.id}")
    print(f"      Periods: {len(model.periods)} quarters")
    print(f"      Nodes: {len(model.nodes)}")
    for node_id, node in model.nodes.items():
        node_type = "calculated" if node.node_type == NodeType.CALCULATED else "mixed/value"
        print(f"         {node_id} ({node_type})")

    # Load the model back
    loaded_model = store.get_statement_model("QUARTERLY_PNL")
    if loaded_model:
        print(f"\n   ✅ Loaded statement model: {loaded_model.id}")
        print(f"      Periods: {len(loaded_model.periods)}")
        print(f"      Nodes: {len(loaded_model.nodes)}")
        print(f"      Has revenue node: {loaded_model.has_node('revenue')}")
        print(f"      Has ebitda node: {loaded_model.has_node('ebitda')}")

    # =========================================================================
    # 9. SCENARIOS
    # =========================================================================
    print("\n🎭 9. Scenarios")
    print("   " + "-" * 30)

    # Create a rate shock scenario
    rate_shock = ScenarioSpec(
        "rate_shock_100bp",
        [
            OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 100.0),
        ],
        name="Rate Shock +100bp",
        description="Parallel shift of +100bp across USD discount curves",
        priority=0,
    )

    # Store the scenario
    store.put_scenario("RATE_SHOCK_100BP", rate_shock)
    print("   ✅ Saved rate shock scenario")

    # Create an equity stress scenario
    equity_stress = ScenarioSpec(
        "equity_crash",
        [
            OperationSpec.equity_price_pct(["SPY", "QQQ"], -20.0),
        ],
        name="Equity Crash -20%",
        description="Market sell-off scenario",
        priority=1,
    )
    store.put_scenario("EQUITY_CRASH", equity_stress)
    print("   ✅ Saved equity crash scenario")

    # Load and display a scenario
    loaded_scenario = store.get_scenario("RATE_SHOCK_100BP")
    if loaded_scenario:
        print(f"   ✅ Loaded scenario: {loaded_scenario.id}")
        print(f"      Name: {loaded_scenario.name}")
        print(f"      Operations: {len(loaded_scenario.operations)}")

    # =========================================================================
    # SUMMARY
    # =========================================================================
    print("\n" + "=" * 70)
    print("🎉 Example Complete!")
    print("=" * 70)
    print("\nKey takeaways:")
    print("  • SqliteStore.open() creates/migrates the database automatically")
    print("  • Use put_*/get_* for individual operations")
    print("  • Use bulk methods (put_*_batch) for efficiency with many records")
    print("  • list_* and latest_*_on_or_before provide historical lookbacks")
    print("  • PortfolioSpec can be created from dicts or typed objects")
    print("  • MetricRegistry provides reusable financial metric definitions")
    print("  • FinancialModelSpec stores complete statement models with nodes/periods")
    print("  • Instruments are stored as tagged union JSON: {'type': ..., 'spec': ...}")
    print("  • All data persists as JSON blobs with SQL indexes for fast lookup")
    print(f"\n📁 Database persisted at: {db_path}")


if __name__ == "__main__":
    main()

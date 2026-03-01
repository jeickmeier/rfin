#!/usr/bin/env python3
"""Portfolio management example demonstrating finstack.portfolio Python bindings.

This example shows how to:
1. Create entities and positions with various instruments
2. Build portfolios using the fluent PortfolioBuilder API
3. Value portfolios and compute metrics
4. Group and aggregate by attributes (sector, rating, etc.)
5. Apply scenarios to portfolios

Run with:
    uv run python finstack-py/examples/scripts/portfolio/portfolio_example.py
"""

from datetime import date

from finstack.core.market_data.context import MarketContext


def build_market_data(as_of: date) -> MarketContext:
    """Create a simple market context with discount curves and prices."""
    market = MarketContext()

    # Discount curves
    from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve

    usd_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9975),
            (1.0, 0.9950),
            (3.0, 0.9750),
            (5.0, 0.9500),
            (10.0, 0.9000),
        ],
    )
    market.insert_discount(usd_curve)

    # Forward curve for floating rate instruments
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,  # 3-month tenor
        [(0.0, 0.0450), (1.0, 0.0475), (3.0, 0.0500), (5.0, 0.0525)],
        base_date=as_of,
    )
    market.insert_forward(forward_curve)

    # Equity prices
    from finstack.core.market_data.scalars import MarketScalar
    from finstack.core.money import Money

    market.insert_price("AAPL-SPOT", MarketScalar.price(Money(185.0, "USD")))
    market.insert_price("MSFT-SPOT", MarketScalar.price(Money(420.0, "USD")))

    return market


def example_1_basic_portfolio() -> None:
    """Example 1: Basic Portfolio Construction."""
    print("\n" + "=" * 80)
    print("Example 1: Basic Portfolio Construction")
    print("=" * 80)

    from finstack.portfolio import Entity, Portfolio

    # Create entities
    entity_a = Entity("ENTITY_A").with_name("Acme Corporation").with_tag("sector", "Technology")

    entity_b = Entity("ENTITY_B").with_name("Beta Industries").with_tag("sector", "Healthcare").with_tag("region", "US")

    print(f"Entity A: {entity_a.id} - {entity_a.name}")
    print(f"  Tags: {entity_a.tags}")
    print(f"Entity B: {entity_b.id} - {entity_b.name}")
    print(f"  Tags: {entity_b.tags}")

    # Create portfolio manually
    from finstack.core.currency import Currency

    portfolio = Portfolio("FUND_001", Currency("USD"), date(2024, 1, 2))
    portfolio.name = "Sample Investment Fund"

    print(f"\nPortfolio ID: {portfolio.id}")
    print(f"Portfolio Name: {portfolio.name}")
    print(f"Base Currency: {portfolio.base_ccy}")
    print(f"As-of Date: {portfolio.as_of}")
    print(f"Entities: {len(portfolio.entities)}")
    print(f"Positions: {len(portfolio.positions)}")


def example_2_portfolio_builder() -> None:
    """Example 2: Portfolio Builder with Instruments."""
    print("\n" + "=" * 80)
    print("Example 2: Portfolio Builder with Instruments")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.dates.daycount import DayCount
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond, Deposit, InterestRateSwap

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit

    as_of = date(2024, 1, 2)

    # Create entities
    entity_corp = Entity("CORP_A").with_name("Corporate A").with_tag("sector", "Finance")

    entity_fund = Entity("FUND_B").with_name("Fund B").with_tag("sector", "Technology")

    # Create instruments
    # 1. Corporate bond
    bond = (
        Bond.builder("BOND_CORP_A")
        .money(Money(5_000_000, "USD"))
        .coupon_rate(0.045)
        .frequency("semiannual")
        .issue(date(2024, 1, 15))
        .maturity(date(2029, 1, 15))
        .disc_id("USD-OIS")
        .build()
    )

    # 2. Money market deposit
    deposit = (
        Deposit.builder("DEPOSIT_MM")
        .money(Money(2_000_000, "USD"))
        .start(as_of)
        .maturity(date(2024, 7, 2))
        .day_count(DayCount.ACT_360)
        .disc_id("USD-OIS")
        .quote_rate(0.0525)
        .build()
    )

    # 3. Interest rate swap
    swap = (
        InterestRateSwap.builder("IRS_USD_5Y")
        .money(Money(10_000_000, "USD"))
        .side("receive_fixed")
        .fixed_rate(0.0425)
        .start(date(2024, 1, 5))
        .maturity(date(2029, 1, 5))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .build()
    )

    # Create positions
    pos_bond = Position(
        "POS_001",
        "CORP_A",
        "BOND_CORP_A",
        bond,
        1.0,  # 1x notional
        PositionUnit.UNITS,
    )

    pos_deposit = Position(
        "POS_002",
        "FUND_B",
        "DEPOSIT_MM",
        deposit,
        1.0,
        PositionUnit.UNITS,
    )

    pos_swap = Position("POS_003", "CORP_A", "IRS_USD_5Y", swap, 1.0, PositionUnit.notional())

    # Build portfolio using builder pattern
    portfolio = (
        PortfolioBuilder("MULTI_ASSET_FUND")
        .name("Multi-Asset Investment Fund")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity([entity_corp, entity_fund])  # Add multiple entities at once
        .position([pos_bond, pos_deposit, pos_swap])  # Add multiple positions at once
        .tag("strategy", "balanced")
        .tag("risk_profile", "moderate")
        .build()
    )

    print(f"Portfolio: {portfolio.id}")
    print(f"  Name: {portfolio.name}")
    print(f"  Entities: {len(portfolio.entities)}")
    print(f"  Positions: {len(portfolio.positions)}")
    print(f"  Tags: {portfolio.tags}")

    # Validate portfolio
    portfolio.validate()
    print("\n✓ Portfolio validation passed")

    # Query positions
    print(f"\nPositions for CORP_A: {len(portfolio.positions_for_entity('CORP_A'))}")
    print(f"Positions for FUND_B: {len(portfolio.positions_for_entity('FUND_B'))}")

    pos = portfolio.get_position("POS_001")
    if pos:
        print("\nPosition POS_001:")
        print(f"  Entity: {pos.entity_id}")
        print(f"  Instrument: {pos.instrument_id}")
        print(f"  Quantity: {pos.quantity}")
        print(f"  Unit: {pos.unit}")
        print(f"  Long position: {pos.is_long()}")


def example_3_portfolio_valuation() -> None:
    """Example 3: Portfolio Valuation and Metrics."""
    print("\n" + "=" * 80)
    print("Example 3: Portfolio Valuation and Metrics")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.dates.daycount import DayCount
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond, Deposit

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, aggregate_metrics, value_portfolio

    as_of = date(2024, 1, 2)
    market = build_market_data(as_of)

    # Create instruments
    bond1 = Bond.builder("BOND_001").money(Money(3_000_000, "USD")).coupon_rate(0.050).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2027, 1, 15)).disc_id("USD-OIS").build()

    bond2 = Bond.builder("BOND_002").money(Money(2_000_000, "USD")).coupon_rate(0.045).frequency("semiannual").issue(date(2024, 2, 1)).maturity(date(2026, 2, 1)).disc_id("USD-OIS").build()

    deposit = Deposit.builder("DEPOSIT_001").money(Money(1_000_000, "USD")).start(as_of).maturity(date(2024, 4, 2)).day_count(DayCount.ACT_360).disc_id("USD-OIS").quote_rate(0.0450).build()

    # Build portfolio
    entity = Entity("TREASURY").with_name("Treasury Department")

    portfolio = (
        PortfolioBuilder("BOND_FUND")
        .name("Fixed Income Fund")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position(Position("POS_B1", "TREASURY", "BOND_001", bond1, 1.0, PositionUnit.UNITS))
        .position(Position("POS_B2", "TREASURY", "BOND_002", bond2, 1.0, PositionUnit.UNITS))
        .position(Position("POS_D1", "TREASURY", "DEPOSIT_001", deposit, 1.0, PositionUnit.UNITS))
        .build()
    )

    print(f"Portfolio: {portfolio.id}")
    print(f"  Positions: {len(portfolio.positions)}")

    # Value the portfolio
    print("\nValuing portfolio...")
    valuation = value_portfolio(portfolio, market)

    print(f"\nTotal Portfolio Value: {valuation.total_base_ccy.format()}")
    print(f"Number of positions valued: {len(valuation.position_values)}")

    # Show position-level values
    print("\nPosition Values:")
    for pos_id, pos_value in valuation.position_values.items():
        print(f"  {pos_id}:")
        print(f"    Native: {pos_value.value_native.format()}")
        print(f"    Base:   {pos_value.value_base.format()}")

    # Entity aggregation
    print("\nEntity Aggregation:")
    for entity_id, value in valuation.by_entity.items():
        print(f"  {entity_id}: {value.format()}")

    # Aggregate metrics
    print("\nAggregating metrics...")
    metrics = aggregate_metrics(valuation, "USD", market)

    print(f"Aggregated metrics: {len(metrics.aggregated)}")
    print(f"Position-level metrics: {len(metrics.by_position)}")

    # Show available metrics
    if metrics.aggregated:
        print("\nAvailable aggregated metrics:")
        for metric_id in list(metrics.aggregated.keys())[:5]:
            metric = metrics.get_metric(metric_id)
            if metric:
                print(f"  {metric.metric_id}: {metric.total:.6f}")


def example_4_grouping_and_aggregation() -> None:
    """Example 4: Attribute-based Grouping and Aggregation."""
    print("\n" + "=" * 80)
    print("Example 4: Attribute-based Grouping and Aggregation")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, value_portfolio

    as_of = date(2024, 1, 2)
    market = build_market_data(as_of)

    # Create bonds with different attributes
    corp_bond_aaa = Bond.builder("CORP_AAA_1").money(Money(2_000_000, "USD")).coupon_rate(0.040).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()

    corp_bond_bbb = Bond.builder("CORP_BBB_1").money(Money(3_000_000, "USD")).coupon_rate(0.055).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()

    treasury = Bond.builder("UST_10Y").money(Money(5_000_000, "USD")).coupon_rate(0.038).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2034, 1, 15)).disc_id("USD-OIS").build()

    # Create entity
    entity = Entity("FIXED_INCOME").with_name("Fixed Income Desk")

    # Note: Position tags must be set via the underlying Rust object.
    # For this example, we'll create a modified portfolio builder approach
    # or demonstrate grouping without tags.

    # Build positions - tags will be added via Rust Position builder in future
    pos1 = Position("POS_CORP_AAA", "FIXED_INCOME", "CORP_AAA_1", corp_bond_aaa, 1.0, PositionUnit.UNITS)
    pos2 = Position("POS_CORP_BBB", "FIXED_INCOME", "CORP_BBB_1", corp_bond_bbb, 1.0, PositionUnit.UNITS)
    pos3 = Position("POS_TREASURY", "FIXED_INCOME", "UST_10Y", treasury, 1.0, PositionUnit.UNITS)

    # Build portfolio
    portfolio = (
        PortfolioBuilder("RATED_PORTFOLIO")
        .name("Credit Portfolio")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position([pos1, pos2, pos3])
        .build()
    )

    print(f"Portfolio: {portfolio.id}")
    print(f"  Total positions: {len(portfolio.positions)}")

    # Note: In this simplified example, positions don't have tags set
    # In a real application, you would set tags when creating Position objects in Rust
    # For now, demonstrate grouping capability with a message

    print("\nGrouping functionality:")
    print("  Note: Position tags would be set via Rust Position::with_tag() method")
    print("  Grouping and aggregation work on any tag key attached to positions")

    # Value the portfolio to show valuation still works
    print("\nValuing portfolio...")
    valuation = value_portfolio(portfolio, market)

    print(f"Total Value: {valuation.total_base_ccy.format()}")

    # Show position values
    print("\nPosition Values:")
    for pos_id, pos_value in valuation.position_values.items():
        print(f"  {pos_id}: {pos_value.value_base.format()}")


def example_5_multi_entity_portfolio() -> None:
    """Example 5: Multi-Entity Portfolio with Various Instruments."""
    print("\n" + "=" * 80)
    print("Example 5: Multi-Entity Portfolio with Various Instruments")
    print("=" * 80)

    from finstack.core.config import FinstackConfig
    from finstack.core.currency import Currency
    from finstack.core.dates.daycount import DayCount
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond, Deposit, Equity, InterestRateSwap

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, aggregate_metrics, value_portfolio

    as_of = date(2024, 1, 2)
    market = build_market_data(as_of)

    # Create multiple entities
    entities = [
        Entity("CORP_TREASURY").with_name("Corporate Treasury").with_tag("department", "treasury"),
        Entity("EQUITY_DESK").with_name("Equity Trading Desk").with_tag("department", "trading"),
        Entity("DERIVATIVES").with_name("Derivatives Desk").with_tag("department", "trading"),
    ]

    # Create diverse instruments
    bond = Bond.builder("CORP_BOND").money(Money(4_000_000, "USD")).coupon_rate(0.048).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2028, 1, 15)).disc_id("USD-OIS").build()

    deposit = Deposit.builder("MM_DEPOSIT").money(Money(1_500_000, "USD")).start(as_of).maturity(date(2024, 3, 2)).day_count(DayCount.ACT_360).disc_id("USD-OIS").quote_rate(0.0475).build()

    equity_aapl = (
        Equity.builder("AAPL_POS")
        .ticker("AAPL")
        .currency(Currency("USD"))
        .shares(5000.0)
        .price_id("AAPL")
        .build()
    )

    equity_msft = (
        Equity.builder("MSFT_POS")
        .ticker("MSFT")
        .currency(Currency("USD"))
        .shares(2000.0)
        .price_id("MSFT")
        .build()
    )

    swap = (
        InterestRateSwap.builder("IRS_PAY_FIXED")
        .money(Money(8_000_000, "USD"))
        .side("pay_fixed")
        .fixed_rate(0.0450)
        .start(date(2024, 1, 5))
        .maturity(date(2027, 1, 5))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .build()
    )

    # Create positions across entities
    positions = [
        Position("POS_BOND", "CORP_TREASURY", "CORP_BOND", bond, 1.0, PositionUnit.UNITS),
        Position("POS_DEP", "CORP_TREASURY", "MM_DEPOSIT", deposit, 1.0, PositionUnit.UNITS),
        Position("POS_AAPL", "EQUITY_DESK", "AAPL_POS", equity_aapl, 1.0, PositionUnit.UNITS),
        Position("POS_MSFT", "EQUITY_DESK", "MSFT_POS", equity_msft, 1.0, PositionUnit.UNITS),
        Position("POS_SWAP", "DERIVATIVES", "IRS_USD_5Y", swap, 1.0, PositionUnit.UNITS),
    ]

    # Build portfolio
    portfolio = (
        PortfolioBuilder("DIVERSIFIED_FUND")
        .name("Diversified Multi-Strategy Fund")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entities)
        .position(positions)
        .tag("fund_type", "hedge_fund")
        .meta("inception_date", "2020-01-01")
        .meta("aum_target", 50_000_000)
        .build()
    )

    print(f"Portfolio: {portfolio.id} - {portfolio.name}")
    print(f"  Entities: {len(portfolio.entities)}")
    print(f"  Positions: {len(portfolio.positions)}")

    # List positions by entity
    print("\nPositions by Entity:")
    for entity_id in portfolio.entities:
        positions = portfolio.positions_for_entity(entity_id)
        print(f"  {entity_id}: {len(positions)} positions")
        for pos in positions:
            print(f"    - {pos.position_id}: {pos.instrument_id}")

    # Value portfolio
    print("\nValuing portfolio...")
    valuation = value_portfolio(portfolio, market, FinstackConfig())

    print(f"\nTotal Portfolio Value: {valuation.total_base_ccy.format()}")

    print("\nValue by Entity:")
    for entity_id, value in valuation.by_entity.items():
        entity_name = portfolio.entities[entity_id].name or entity_id
        print(f"  {entity_name}: {value.format()}")

    # Compute metrics
    metrics = aggregate_metrics(valuation, "USD", market)

    # Show portfolio-level metrics if available
    if metrics.aggregated:
        print("\nPortfolio Metrics (aggregated):")
        for metric_id, metric in list(metrics.aggregated.items())[:3]:
            print(f"  {metric_id}: {metric.total:.6f}")


def example_6_portfolio_results() -> None:
    """Example 6: Complete Portfolio Results."""
    print("\n" + "=" * 80)
    print("Example 6: Complete Portfolio Results")
    print("=" * 80)

    from finstack.core.config import FinstackConfig
    from finstack.core.currency import Currency
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, aggregate_metrics, value_portfolio

    as_of = date(2024, 1, 2)
    market = build_market_data(as_of)
    config = FinstackConfig()

    # Simple portfolio
    bond = Bond.builder("BOND_SIMPLE").money(Money(10_000_000, "USD")).coupon_rate(0.045).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()

    entity = Entity("TREASURY").with_name("Treasury")

    portfolio = (
        PortfolioBuilder("SIMPLE_FUND")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position(Position("POS_001", "TREASURY", "BOND_SIMPLE", bond, 1.0, PositionUnit.UNITS))
        .build()
    )

    # Get valuation and metrics
    valuation = value_portfolio(portfolio, market, config)
    metrics = aggregate_metrics(valuation, "USD", market)

    print(f"Portfolio Results for: {portfolio.id}")
    print(f"\nTotal Value: {valuation.total_base_ccy.format()}")

    # Access metrics
    print("\nPortfolio Metrics:")
    for metric_id in list(metrics.aggregated.keys())[:5]:
        metric = metrics.get_metric(metric_id)
        if metric:
            print(f"  {metric.metric_id}: {metric.total:.6f}")

    # Show entity breakdown
    print("\nEntity Breakdown:")
    for entity_id, value in valuation.by_entity.items():
        print(f"  {entity_id}: {value.format()}")


def example_7_position_units() -> None:
    """Example 7: Different Position Units."""
    print("\n" + "=" * 80)
    print("Example 7: Different Position Units")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond, Equity

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit

    as_of = date(2024, 1, 2)

    # Create instruments
    bond = Bond.builder("BOND_FACE").money(Money(1_000_000, "USD")).coupon_rate(0.050).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()

    equity = (
        Equity.builder("EQUITY_UNITS")
        .ticker("AAPL")
        .currency(Currency("USD"))
        .shares(100.0)
        .price_id("AAPL")
        .build()
    )

    entity = Entity("PORTFOLIO_MGR")

    # Different position units
    positions = [
        Position("POS_FACE", "PORTFOLIO_MGR", "BOND_FACE", bond, 10_000_000, PositionUnit.FACE_VALUE),
        Position("POS_UNITS", "PORTFOLIO_MGR", "EQUITY_UNITS", equity, 5000.0, PositionUnit.UNITS),
        Position(
            "POS_NOTIONAL",
            "PORTFOLIO_MGR",
            "BOND_FACE",
            bond,
            5_000_000,
            PositionUnit.notional_with_ccy(Currency("USD")),
        ),
        Position("POS_PCT", "PORTFOLIO_MGR", "EQUITY_UNITS", equity, 0.001, PositionUnit.PERCENTAGE),
    ]

    portfolio = (
        PortfolioBuilder("UNIT_DEMO").base_ccy(Currency("USD")).as_of(as_of).entity(entity).position(positions).build()
    )

    print(f"Portfolio: {portfolio.id}")
    print("\nPosition Units:")
    for pos in portfolio.positions:
        print(f"  {pos.position_id}:")
        print(f"    Instrument: {pos.instrument_id}")
        print(f"    Quantity: {pos.quantity:,.2f}")
        print(f"    Unit: {pos.unit}")
        print(f"    Long: {pos.is_long()}")


def example_8_scenario_integration() -> None:
    """Example 8: Portfolio Scenario Analysis (if scenarios feature enabled)."""
    print("\n" + "=" * 80)
    print("Example 8: Portfolio Scenario Analysis")
    print("=" * 80)

    try:
        from finstack.core.currency import Currency
        from finstack.core.money import Money
        from finstack.valuations.instruments import Bond

        from finstack.portfolio import (
            Entity,
            PortfolioBuilder,
            Position,
            PositionUnit,
            apply_and_revalue,
            value_portfolio,
        )
        from finstack.scenarios import CurveKind, OperationSpec, ScenarioSpec
    except ImportError:
        print("  ⚠ Scenarios feature not enabled - skipping")
        return

    as_of = date(2024, 1, 2)
    market = build_market_data(as_of)

    # Create portfolio
    bond = Bond.builder("BOND_RATE_SENS").money(Money(10_000_000, "USD")).coupon_rate(0.045).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2034, 1, 15)).disc_id("USD-OIS").build()

    entity = Entity("TREASURY")

    portfolio = (
        PortfolioBuilder("RATE_SENSITIVE")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position(Position("POS_BOND", "TREASURY", "BOND_RATE_SENS", bond, 1.0, PositionUnit.UNITS))
        .build()
    )

    # Base case valuation
    print("Base case valuation...")
    base_valuation = value_portfolio(portfolio, market)
    print(f"  Base case value: {base_valuation.total_base_ccy.format()}")

    # Create rate shock scenario (+50 bps)
    rate_shock = ScenarioSpec(
        "rate_shock_50bp",
        [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 50.0)],
        name="Rate Shock +50bp",
        description="Parallel shift of USD discount curve by 50 basis points",
    )

    print(f"\nApplying scenario: {rate_shock.name}")
    print(f"  Operations: {len(rate_shock.operations)}")

    # Apply scenario and revalue
    shocked_valuation = apply_and_revalue(portfolio, rate_shock, market)

    print(f"\nShocked valuation: {shocked_valuation.total_base_ccy.format()}")

    # Compare
    base_value = base_valuation.total_base_ccy.amount
    shocked_value = shocked_valuation.total_base_ccy.amount
    change = shocked_value - base_value
    change_pct = (change / base_value) * 100 if base_value != 0 else 0

    print("\nImpact Analysis:")
    print(f"  Base value:    ${base_value:,.2f}")
    print(f"  Shocked value: ${shocked_value:,.2f}")
    print(f"  Change:        ${change:,.2f} ({change_pct:+.2f}%)")


def example_9_long_short_positions() -> None:
    """Example 9: Long and Short Positions."""
    print("\n" + "=" * 80)
    print("Example 9: Long and Short Positions")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit

    as_of = date(2024, 1, 2)

    bond = Bond.builder("BOND_LS").money(Money(1_000_000, "USD")).coupon_rate(0.045).frequency("semiannual").issue(date(2024, 1, 15)).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()

    entity = Entity("HEDGE_FUND").with_name("Hedge Fund Desk")

    # Create long and short positions
    long_position = Position("POS_LONG", "HEDGE_FUND", "BOND_LS", bond, 5.0, PositionUnit.UNITS)  # Long 5x notional

    short_position = Position(
        "POS_SHORT",
        "HEDGE_FUND",
        "BOND_LS",
        bond,
        -2.0,
        PositionUnit.UNITS,  # Short 2x notional
    )

    portfolio = (
        PortfolioBuilder("LONG_SHORT")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position([long_position, short_position])
        .build()
    )

    print(f"Portfolio: {portfolio.id}")
    print("\nPositions:")
    for pos in portfolio.positions:
        direction = "LONG" if pos.is_long() else "SHORT"
        print(f"  {pos.position_id}: {direction} {abs(pos.quantity):.1f}x {pos.instrument_id}")


def example_10_dummy_entity() -> None:
    """Example 10: Standalone Instruments with Dummy Entity."""
    print("\n" + "=" * 80)
    print("Example 10: Standalone Instruments with Dummy Entity")
    print("=" * 80)

    from finstack.core.currency import Currency
    from finstack.core.dates.daycount import DayCount
    from finstack.core.money import Money
    from finstack.valuations.instruments import Deposit, InterestRateSwap

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit

    as_of = date(2024, 1, 2)

    # For standalone derivatives that don't belong to a specific entity
    dummy = Entity.dummy()

    print(f"Dummy entity ID: {dummy.id}")
    print(f"Dummy entity name: {dummy.name}")

    # Standalone instruments
    swap1 = (
        InterestRateSwap.builder("SWAP_STANDALONE_1")
        .money(Money(5_000_000, "USD"))
        .side("receive_fixed")
        .fixed_rate(0.0425)
        .start(date(2024, 1, 5))
        .maturity(date(2027, 1, 5))
        .disc_id("USD-OIS")
        .fwd_id("USD-SOFR-3M")
        .build()
    )

    deposit1 = Deposit.builder("DEP_STANDALONE").money(Money(1_000_000, "USD")).start(as_of).maturity(date(2024, 4, 2)).day_count(DayCount.ACT_360).disc_id("USD-OIS").quote_rate(0.0450).build()

    # Positions referencing dummy entity
    positions = [
        Position("POS_SWAP_1", dummy.id, "SWAP_STANDALONE_1", swap1, 1.0, PositionUnit.UNITS),
        Position("POS_DEP_1", dummy.id, "DEP_STANDALONE", deposit1, 1.0, PositionUnit.UNITS),
    ]

    portfolio = (
        PortfolioBuilder("STANDALONE_INSTRUMENTS")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(dummy)
        .position(positions)
        .build()
    )

    print(f"\nPortfolio: {portfolio.id}")
    print(f"  Positions: {len(portfolio.positions)}")
    print(f"  All positions use dummy entity: {dummy.id}")

    for pos in portfolio.positions:
        print(f"    - {pos.position_id}: entity={pos.entity_id}, instrument={pos.instrument_id}")


def main() -> None:
    """Run all portfolio examples."""
    print("\n" + "#" * 80)
    print("# FINSTACK PORTFOLIO MANAGEMENT EXAMPLES")
    print("#" * 80)
    print("\nDemonstrating portfolio capabilities:")
    print("  • Entity and position management")
    print("  • Fluent portfolio builder API")
    print("  • Portfolio valuation and metrics")
    print("  • Attribute-based grouping and aggregation")
    print("  • Multi-entity portfolios")
    print("  • Long/short positions")
    print("  • Scenario analysis")

    example_1_basic_portfolio()
    example_2_portfolio_builder()
    example_3_portfolio_valuation()
    example_4_grouping_and_aggregation()
    example_5_multi_entity_portfolio()
    example_6_portfolio_results()
    example_7_position_units()
    example_8_scenario_integration()
    example_9_long_short_positions()
    example_10_dummy_entity()

    print("\n" + "#" * 80)
    print("# All portfolio examples completed successfully!")
    print("#" * 80)


if __name__ == "__main__":
    main()

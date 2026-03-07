"""Title: Scenario-Based Stress Testing
Persona: Portfolio Manager, Risk Analyst
Complexity: Intermediate
Runtime: ~3 seconds.

Description:
Demonstrates a stress testing workflow:
- Build a base portfolio (rates + credit)
- Define stress scenarios (rate + credit)
- Apply scenarios using ScenarioEngine
- Compare before/after valuations and compute P&L
"""

from datetime import date, timedelta

from finstack.core.currency import Currency
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, value_portfolio
from finstack.scenarios.builder import scenario
from finstack.scenarios import ExecutionContext, ScenarioEngine
from finstack.statements.types import FinancialModelSpec
from finstack.valuations.instruments import Bond, CreditDefaultSwap


def create_base_market(as_of: date) -> MarketContext:
    """Create baseline market data."""
    market = MarketContext()

    # Discount curves
    market.insert(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))

    # Credit curves (for CDS)
    market.insert(HazardCurve("ACME.5Y", as_of, [(0.0, 0.015), (10.0, 0.015)], recovery_rate=0.40))

    return market


def create_portfolio():
    """Create test portfolio with mixed instruments."""
    as_of = date(2024, 1, 15)

    fund = Entity("FUND-001").with_name("Multi-Strategy Fund").with_tag("strategy", "multi-asset")

    # 1. Corporate bonds (interest rate sensitive)
    bond1 = (
        Bond.builder("CORP.5Y")
        .money(Money(10_000_000, "USD"))
        .coupon_rate(0.05)
        .frequency("semiannual")
        .issue(date(2024, 1, 15))
        .maturity(date(2029, 1, 15))
        .disc_id("USD-OIS")
        .build()
    )
    pos_bond = Position("POS-BOND-1", fund.id, "CORP.5Y", bond1, 1.0, PositionUnit.UNITS).with_tag("asset_class", "rates")

    # 2. CDS (credit sensitive)
    cds = CreditDefaultSwap.buy_protection(
        "ACME.CDS.5Y",
        Money(5_000_000, "USD"),
        spread_bp=150.0,
        start_date=as_of + timedelta(days=1),
        maturity=date(2029, 1, 15),
        discount_curve="USD-OIS",
        credit_curve="ACME.5Y",
    )
    pos_cds = Position("POS-CDS-1", fund.id, "ACME.CDS.5Y", cds, 1.0, PositionUnit.UNITS).with_tag("asset_class", "credit")

    return (
        PortfolioBuilder("STRESS_TEST_PORTFOLIO")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(fund)
        .position([pos_bond, pos_cds])
        .build()
    )


def define_scenarios():
    """Define stress scenarios."""
    scenarios = {}

    # 1. Rate shock: +100bp parallel shift
    scenarios["RATE_UP_100BP"] = (
        scenario("Rate Shock +100bp")
        .description("Parallel +100bp shift to all discount curves")
        .shift_discount_curve("USD-OIS", 100)  # +100bp
        .build()
    )

    # 2. Credit widening: +200bp to credit spreads
    scenarios["CREDIT_WIDEN_200BP"] = (
        scenario("Credit Widening +200bp")
        .description("Parallel +200bp shift to credit curves")
        .shift_hazard_curve("ACME.5Y", 200)  # +200bp
        .build()
    )

    # 3. Combined stress: Rates up + credit wide
    scenarios["COMBINED_STRESS"] = (
        scenario("Combined Stress")
        .description("Multi-factor stress: rates +50bp, credit +100bp")
        .shift_discount_curve("USD-OIS", 50)
        .shift_hazard_curve("ACME.5Y", 100)
        .build()
    )

    return scenarios


def main() -> None:
    """Run stress testing workflow."""
    # 1. Create base market and portfolio
    as_of = date(2024, 1, 15)
    market = create_base_market(as_of)
    portfolio = create_portfolio()

    # 2. Value at baseline
    baseline_result = value_portfolio(portfolio, market)
    baseline_pv = baseline_result.total_base_ccy.amount

    # 3. Define scenarios
    scenarios = define_scenarios()
    for _name, _spec in scenarios.items():
        pass

    # 4. Run stress tests

    engine = ScenarioEngine()
    results = {}

    for scenario_name, scenario_spec in scenarios.items():
        # Apply scenario to a fresh execution context (mutated in-place)
        ctx = ExecutionContext(market.clone(), FinancialModelSpec("empty", []), as_of)
        _report = engine.apply(scenario_spec, ctx)
        shocked_market = ctx.market

        # Revalue portfolio
        stressed_result = value_portfolio(portfolio, shocked_market)
        stressed_pv = stressed_result.total_base_ccy.amount

        # Calculate impact
        pnl = stressed_pv - baseline_pv
        pnl_pct = (pnl / baseline_pv) * 100

        results[scenario_name] = {"pv": stressed_pv, "pnl": pnl, "pnl_pct": pnl_pct}

    # 5. Summary comparison
    for scenario_name in scenarios:
        pnl = results[scenario_name]["pnl"]
        pnl_pct = results[scenario_name]["pnl_pct"]

        # Color code (if terminal supports it)

    # 6. Worst case scenario
    min(results.items(), key=lambda x: x[1]["pnl"])

    # Best case scenario
    max(results.items(), key=lambda x: x[1]["pnl"])

    # 7. Export to DataFrame for further analysis

    # Create comparison DataFrame
    import polars as pl

    scenario_data = []
    for scenario_name, res in results.items():
        scenario_data.append({
            "scenario": scenario_name,
            "baseline_pv": baseline_pv,
            "stressed_pv": res["pv"],
            "pnl": res["pnl"],
            "pnl_pct": res["pnl_pct"],
        })

    pl.DataFrame(scenario_data)

    # Can export to CSV/Parquet
    # df.write_csv("stress_test_results.csv")
    # df.write_parquet("stress_test_results.parquet")


if __name__ == "__main__":
    main()

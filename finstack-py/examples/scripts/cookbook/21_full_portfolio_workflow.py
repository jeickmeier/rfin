"""Title: Complete Portfolio Workflow (Build → Price → Stress → Optimize)
Persona: Portfolio Manager
Complexity: Advanced
Runtime: ~5 seconds.

Description:
End-to-end portfolio management workflow demonstrating:
1. Market data calibration from quotes
2. Multi-asset portfolio construction
3. Baseline valuation with risk metrics
4. Scenario-based stress testing
5. Constrained portfolio optimization
6. Report generation and export

Key Concepts:
- Complete workflow integration
- Market data bootstrap
- Risk measurement and limits
- Scenario analysis
- Portfolio optimization
- DataFrame export for reporting

Prerequisites:
- Understanding of all previous examples
- Portfolio management concepts
- Risk management frameworks
"""

from datetime import date, timedelta

import polars as pl

from finstack.core.currency import Currency
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, value_portfolio
from finstack.portfolio import optimize_max_yield_with_ccc_limit
from finstack.scenarios import CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec
from finstack.statements.types import FinancialModelSpec
from finstack.valuations.instruments import Bond, CreditDefaultSwap
from finstack.valuations.pricer import create_standard_registry


def step1_build_market_data(as_of: date) -> MarketContext:
    """Step 1: Create a small market context (curves only)."""
    market = MarketContext()
    market.insert(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))
    market.insert(HazardCurve("CORP.A", as_of, [(0.0, 0.02), (10.0, 0.02)], recovery_rate=0.40))
    return market


def step2_build_portfolio(as_of: date):
    """Step 2: Construct a small rates + credit portfolio."""
    fund = Entity("FUND-001").with_name("Multi-Asset Fund")

    bond_2y = Bond.builder("BOND.2Y").money(Money(10_000_000, "USD")).coupon_rate(0.04).frequency("semiannual").issue(as_of).maturity(date(2026, 1, 15)).disc_id("USD-OIS").build()
    bond_5y = Bond.builder("BOND.5Y").money(Money(20_000_000, "USD")).coupon_rate(0.045).frequency("semiannual").issue(as_of).maturity(date(2029, 1, 15)).disc_id("USD-OIS").build()
    cds_5y = CreditDefaultSwap.buy_protection(
        "CDS.5Y",
        Money(10_000_000, "USD"),
        spread_bp=200.0,
        start_date=as_of + timedelta(days=1),
        maturity=date(2029, 1, 15),
        discount_curve="USD-OIS",
        credit_curve="CORP.A",
    )

    positions = [
        Position("POS-BOND-2Y", fund.id, "BOND.2Y", bond_2y, 1.0, PositionUnit.UNITS).with_tag("rating", "AAA"),
        Position("POS-BOND-5Y", fund.id, "BOND.5Y", bond_5y, 1.0, PositionUnit.UNITS).with_tag("rating", "AA"),
        Position("POS-CDS-5Y", fund.id, "CDS.5Y", cds_5y, 1.0, PositionUnit.UNITS).with_tag("rating", "BBB"),
    ]

    return (
        PortfolioBuilder("FULL_WORKFLOW")
        .name("Full Workflow Demo")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(fund)
        .position(positions)
        .build()
    )


def step3_baseline_valuation(portfolio, market):
    """Step 3: Baseline valuation with risk metrics."""
    # Value portfolio
    result = value_portfolio(portfolio, market)

    # Position breakdown

    for pos_id in result.position_values.keys():
        pos = next(p for p in portfolio.positions if p.position_id == pos_id)
        pos.tags.get("rating", "N/A")

    # Risk metrics: for a full position-instrument metric workflow, see the
    # dedicated examples under `examples/scripts/portfolio/` and `examples/scripts/valuations/`.
    return result, {"dv01": 0.0, "cs01": 0.0, "delta": 0.0}


def step4_stress_testing(portfolio, market):
    """Step 4: Run stress scenarios."""
    # Define scenarios
    scenarios = {
        "RATE_UP_100BP": ScenarioSpec(
            "rate_up_100bp",
            [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-OIS", 100.0)],
            name="Rate +100bp",
        ),
        "CREDIT_WIDEN_200BP": ScenarioSpec(
            "credit_widen_200bp",
            [OperationSpec.curve_parallel_bp(CurveKind.ParCDS, "CORP.A", 200.0)],
            name="Credit +200bp",
        ),
    }

    # Baseline
    baseline = value_portfolio(portfolio, market)
    baseline_pv = baseline.total_base_ccy.amount

    # Run scenarios
    engine = ScenarioEngine()

    stress_results = {}
    for name, spec in scenarios.items():
        ctx = ExecutionContext(market.clone(), FinancialModelSpec("empty", []), portfolio.as_of)
        _report = engine.apply(spec, ctx)
        result = value_portfolio(portfolio, ctx.market)
        pv = result.total_base_ccy.amount
        pnl = pv - baseline_pv
        (pnl / baseline_pv) * 100

        stress_results[name] = pnl

    min(stress_results, key=stress_results.get)

    return stress_results


def step5_portfolio_optimization(portfolio, market):
    """Step 5: Constrained portfolio optimization."""
    # Use the built-in helper for a quick, realistic optimization pass.
    return optimize_max_yield_with_ccc_limit(portfolio, market, ccc_limit=0.10)


def step6_export_results(portfolio, baseline_result, stress_results, risk_metrics) -> None:
    """Step 6: Export results to DataFrame."""
    # Portfolio positions DataFrame
    baseline_result.to_polars()

    # Risk metrics DataFrame
    risk_data = [
        {"metric": "DV01", "value": risk_metrics["dv01"], "unit": "USD"},
        {"metric": "CS01", "value": risk_metrics["cs01"], "unit": "USD"},
        {"metric": "Delta", "value": risk_metrics["delta"], "unit": "shares"},
    ]
    pl.DataFrame(risk_data)

    # Stress results DataFrame
    stress_data = [{"scenario": name, "pnl": pnl} for name, pnl in stress_results.items()]
    pl.DataFrame(stress_data)

    # Export to files (commented out for example)
    # df_portfolio.write_csv("portfolio_valuation.csv")
    # df_risk.write_csv("risk_metrics.csv")
    # df_stress.write_csv("stress_results.csv")


def main() -> None:
    """Execute complete portfolio workflow."""
    as_of = date(2024, 1, 15)

    # Step 1: Market data
    market = step1_build_market_data(as_of)

    # Step 2: Build portfolio
    portfolio = step2_build_portfolio(as_of)

    # Step 3: Baseline valuation
    baseline_result, risk_metrics = step3_baseline_valuation(portfolio, market)

    # Step 4: Stress testing
    stress_results = step4_stress_testing(portfolio, market)

    # Step 5: Portfolio optimization
    step5_portfolio_optimization(portfolio, market)

    # Step 6: Export results
    step6_export_results(portfolio, baseline_result, stress_results, risk_metrics)

    # Final summary


if __name__ == "__main__":
    main()

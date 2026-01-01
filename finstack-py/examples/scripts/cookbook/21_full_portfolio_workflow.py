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

import polars as pl

from finstack import (
    # Instruments
    Bond,
    Book,
    BookId,
    CandidatePosition,
    Constraint,
    CreditDefaultSwap,
    CreditQuote,
    # Market data
    Date,
    Entity,
    EquityOption,
    Money,
    Objective,
    # Portfolio
    PortfolioBuilder,
    # Optimization
    PortfolioOptimizationProblem,
    RatesQuote,
    # Risk
    ScenarioEngine,
    TradeUniverse,
    VolSurface,
    # Pricing
    create_standard_registry,
    execute_calibration_v2,
    value_portfolio,
)
from finstack.scenarios import scenario


def step1_calibrate_market_data():
    """Step 1: Bootstrap market curves from quotes."""
    base_date = Date(2024, 1, 15)

    # Create quotes
    ois_quotes = [
        RatesQuote.deposit("ON", 0.0525, base_date, "USD.OIS"),
        RatesQuote.ois_swap("3M", 0.0540, base_date, "USD.OIS"),
        RatesQuote.ois_swap("1Y", 0.0475, base_date, "USD.OIS"),
        RatesQuote.ois_swap("2Y", 0.0455, base_date, "USD.OIS"),
        RatesQuote.ois_swap("5Y", 0.0435, base_date, "USD.OIS"),
        RatesQuote.ois_swap("10Y", 0.0425, base_date, "USD.OIS"),
    ]

    cds_quotes = [
        CreditQuote.cds_par_spread("1Y", 0.0120, base_date, "CORP.A"),
        CreditQuote.cds_par_spread("3Y", 0.0180, base_date, "CORP.A"),
        CreditQuote.cds_par_spread("5Y", 0.0200, base_date, "CORP.A"),
    ]

    # Calibration plan
    plan = {
        "base_date": base_date.to_dict(),
        "steps": [
            {
                "kind": "discount",
                "id": "USD.OIS",
                "quotes": [q.to_dict() for q in ois_quotes],
                "interpolation": "log_linear",
                "day_count": "Act360",
            },
            {
                "kind": "hazard",
                "id": "CORP.A",
                "quotes": [q.to_dict() for q in cds_quotes],
                "discount_curve_id": "USD.OIS",
                "recovery_rate": 0.40,
                "interpolation": "log_linear",
            },
        ],
    }

    # Execute calibration
    market, _report = execute_calibration_v2(plan)

    # Add additional market data
    market.set_equity("SPY", 480.0)

    spy_vol = VolSurface.flat("SPY.VOL", 0.18, "lognormal")
    market.set_vol_surface(spy_vol)

    return market


def step2_build_portfolio(market):
    """Step 2: Construct multi-asset portfolio."""
    builder = PortfolioBuilder()
    builder.base_currency("USD")
    builder.as_of(Date(2024, 1, 15))

    # Entities
    fund = Entity(id="FUND-001", name="Multi-Asset Fund")
    builder.entity(fund)

    # Books
    rates_book = Book(id=BookId("RATES"), name="Rates Portfolio")
    credit_book = Book(id=BookId("CREDIT"), name="Credit Portfolio")
    equity_book = Book(id=BookId("EQUITY"), name="Equity Portfolio")
    builder.books([rates_book, credit_book, equity_book])

    # Instruments
    instruments = [
        # Bonds
        (
            "BOND.2Y",
            Bond.fixed_semiannual(
                "BOND.2Y", Money.from_code(10_000_000, "USD"), 0.04, Date(2024, 1, 15), Date(2026, 1, 15), "USD.OIS"
            ),
            BookId("RATES"),
            {"rating": "AAA"},
        ),
        (
            "BOND.5Y",
            Bond.fixed_semiannual(
                "BOND.5Y", Money.from_code(20_000_000, "USD"), 0.045, Date(2024, 1, 15), Date(2029, 1, 15), "USD.OIS"
            ),
            BookId("RATES"),
            {"rating": "AA"},
        ),
        # CDS
        (
            "CDS.5Y",
            CreditDefaultSwap(
                "CDS.5Y",
                Money.from_code(10_000_000, "USD"),
                0.020,
                Date(2024, 1, 15),
                Date(2029, 1, 15),
                True,
                "CORP.A",
                "USD.OIS",
            ),
            BookId("CREDIT"),
            {"rating": "BBB"},
        ),
        # Equity options
        (
            "CALL.SPY",
            EquityOption.european("CALL.SPY", 500.0, Date(2024, 7, 15), True, "SPY", 100.0, "USD.OIS"),
            BookId("EQUITY"),
            {"rating": "N/A"},
        ),
    ]

    for pos_id, instrument, book_id, tags in instruments:
        builder.position(id=pos_id, instrument=instrument, entity_id=fund.id, quantity=1.0, tags=tags)
        builder.add_position_to_book(pos_id, book_id)

    portfolio = builder.build()

    return portfolio


def step3_baseline_valuation(portfolio, market):
    """Step 3: Baseline valuation with risk metrics."""
    # Value portfolio
    result = value_portfolio(portfolio, market, None)

    # Position breakdown

    for pos_val in result.position_values:
        pos = next(p for p in portfolio.positions() if p.id == pos_val.position_id)
        pos.tags.get("rating", "N/A")

    # Compute risk metrics
    registry = create_standard_registry()

    total_dv01 = 0.0
    total_cs01 = 0.0
    total_delta = 0.0

    for position in portfolio.positions():
        instrument = position.instrument

        # Determine metrics and pricing method
        if isinstance(instrument, Bond):
            res = registry.price_with_metrics(instrument, "discounting", market, ["dv01"])
            dv01 = res.measures.get("dv01") or 0.0
            total_dv01 += dv01

        elif isinstance(instrument, CreditDefaultSwap):
            res = registry.price_with_metrics(instrument, "discounting", market, ["cs01", "dv01"])
            cs01 = res.measures.get("cs01") or 0.0
            dv01 = res.measures.get("dv01") or 0.0
            total_cs01 += cs01
            total_dv01 += dv01

        elif isinstance(instrument, EquityOption):
            res = registry.price_with_metrics(instrument, "black_scholes", market, ["delta"])
            delta = res.measures.get("delta") or 0.0
            total_delta += delta * position.quantity

    return result, {"dv01": total_dv01, "cs01": total_cs01, "delta": total_delta}


def step4_stress_testing(portfolio, market):
    """Step 4: Run stress scenarios."""
    # Define scenarios
    scenarios = {
        "RATE_UP_100BP": scenario("Rate +100bp").shift_discount_curve("USD.OIS", 100).build(),
        "CREDIT_WIDEN_200BP": scenario("Credit +200bp").shift_hazard_curve("CORP.A", 200).build(),
        "EQUITY_CRASH_20PCT": scenario("Equity -20%").shift_equity("SPY", -20.0).build(),
    }

    # Baseline
    baseline = value_portfolio(portfolio, market, None)
    baseline_pv = baseline.total.amount

    # Run scenarios
    engine = ScenarioEngine()

    stress_results = {}
    for name, spec in scenarios.items():
        from datetime import date

        from finstack.scenarios import ExecutionContext
        from finstack.statements.types import FinancialModelSpec

        ctx = ExecutionContext(market.clone(), FinancialModelSpec("empty", []), date.today())
        _ = engine.apply(spec, ctx)
        shocked_market = ctx.market
        result = value_portfolio(portfolio, shocked_market, None)
        pv = result.total.amount
        pnl = pv - baseline_pv
        (pnl / baseline_pv) * 100

        stress_results[name] = pnl

    min(stress_results, key=stress_results.get)

    return stress_results


def step5_portfolio_optimization(market):
    """Step 5: Constrained portfolio optimization."""
    # Define trade universe (candidate positions)
    candidates = [
        CandidatePosition(id="BOND.AAA", instrument_type="bond", tags={"rating": "AAA"}, expected_yield=0.040),
        CandidatePosition(id="BOND.AA", instrument_type="bond", tags={"rating": "AA"}, expected_yield=0.045),
        CandidatePosition(id="BOND.BBB", instrument_type="bond", tags={"rating": "BBB"}, expected_yield=0.050),
        CandidatePosition(id="BOND.BB", instrument_type="bond", tags={"rating": "BB"}, expected_yield=0.070),
        CandidatePosition(id="BOND.CCC", instrument_type="bond", tags={"rating": "CCC"}, expected_yield=0.100),
    ]

    universe = TradeUniverse(candidates)

    # Create optimization problem
    problem = PortfolioOptimizationProblem(universe)

    # Objective: maximize yield
    problem.add_objective(Objective.maximize_metric("expected_yield"))

    # Constraints
    problem.add_constraint(Constraint.budget(100_000_000))  # $100M total
    problem.add_constraint(Constraint.weight_bounds(0.0, 0.25))  # Max 25% per position
    problem.add_constraint(Constraint.tag_exposure_limit("rating", "CCC", 0.10))  # Max 10% CCC
    problem.add_constraint(Constraint.tag_exposure_minimum("rating", "AAA", 0.20))  # Min 20% AAA

    # Solve
    opt_result = problem.solve()

    for trade in opt_result.trades:
        candidate = next(c for c in candidates if c.id == trade.position_id)
        rating = candidate.tags.get("rating", "N/A")

    # Rating distribution
    from collections import defaultdict

    rating_dist = defaultdict(float)
    for trade in opt_result.trades:
        candidate = next(c for c in candidates if c.id == trade.position_id)
        rating = candidate.tags.get("rating", "N/A")
        rating_dist[rating] += trade.target_weight

    for rating, _weight in sorted(rating_dist.items()):
        pass

    # Verify constraints

    return opt_result


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
    # Step 1: Calibrate market data
    market = step1_calibrate_market_data()

    # Step 2: Build portfolio
    portfolio = step2_build_portfolio(market)

    # Step 3: Baseline valuation
    baseline_result, risk_metrics = step3_baseline_valuation(portfolio, market)

    # Step 4: Stress testing
    stress_results = step4_stress_testing(portfolio, market)

    # Step 5: Portfolio optimization
    step5_portfolio_optimization(market)

    # Step 6: Export results
    step6_export_results(portfolio, baseline_result, stress_results, risk_metrics)

    # Final summary


if __name__ == "__main__":
    main()

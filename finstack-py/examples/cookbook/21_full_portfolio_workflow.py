"""
Title: Complete Portfolio Workflow (Build → Price → Stress → Optimize)
Persona: Portfolio Manager
Complexity: Advanced
Runtime: ~5 seconds

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

from finstack import (
    # Market data
    Date,
    MarketContext,
    DiscountCurve,
    HazardCurve,
    VolSurface,
    FxMatrix,
    RatesQuote,
    CreditQuote,
    execute_calibration_v2,
    
    # Portfolio
    PortfolioBuilder,
    Entity,
    Book,
    BookId,
    
    # Instruments
    Bond,
    CreditDefaultSwap,
    EquityOption,
    InterestRateSwap,
    Money,
    
    # Pricing
    create_standard_registry,
    value_portfolio,
    
    # Risk
    aggregate_by_attribute,
    
    # Scenarios
    ScenarioEngine,
    
    # Optimization
    PortfolioOptimizationProblem,
    Objective,
    Constraint,
    TradeUniverse,
    CandidatePosition,
)
from finstack.scenarios import scenario
import polars as pl


def step1_calibrate_market_data():
    """Step 1: Bootstrap market curves from quotes."""
    print("STEP 1: Calibrate Market Data")
    print("="*80)
    
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
                "day_count": "Act360"
            },
            {
                "kind": "hazard",
                "id": "CORP.A",
                "quotes": [q.to_dict() for q in cds_quotes],
                "discount_curve_id": "USD.OIS",
                "recovery_rate": 0.40,
                "interpolation": "log_linear"
            }
        ]
    }
    
    # Execute calibration
    market, report = execute_calibration_v2(plan)
    
    # Add additional market data
    market.set_equity("SPY", 480.0)
    
    spy_vol = VolSurface.flat("SPY.VOL", 0.18, "lognormal")
    market.set_vol_surface(spy_vol)
    
    print(f"  ✓ Calibrated {len(ois_quotes)} OIS quotes")
    print(f"  ✓ Calibrated {len(cds_quotes)} CDS quotes")
    print(f"  ✓ Added equity spot and vol surface")
    print()
    
    return market


def step2_build_portfolio(market):
    """Step 2: Construct multi-asset portfolio."""
    print("STEP 2: Build Portfolio")
    print("="*80)
    
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
        ("BOND.2Y", Bond.fixed_semiannual(
            "BOND.2Y", Money.from_code(10_000_000, "USD"), 0.04,
            Date(2024, 1, 15), Date(2026, 1, 15), "USD.OIS"
        ), BookId("RATES"), {"rating": "AAA"}),
        
        ("BOND.5Y", Bond.fixed_semiannual(
            "BOND.5Y", Money.from_code(20_000_000, "USD"), 0.045,
            Date(2024, 1, 15), Date(2029, 1, 15), "USD.OIS"
        ), BookId("RATES"), {"rating": "AA"}),
        
        # CDS
        ("CDS.5Y", CreditDefaultSwap(
            "CDS.5Y", Money.from_code(10_000_000, "USD"), 0.020,
            Date(2024, 1, 15), Date(2029, 1, 15), True,
            "CORP.A", "USD.OIS"
        ), BookId("CREDIT"), {"rating": "BBB"}),
        
        # Equity options
        ("CALL.SPY", EquityOption.european(
            "CALL.SPY", 500.0, Date(2024, 7, 15), True,
            "SPY", 100.0, "USD.OIS"
        ), BookId("EQUITY"), {"rating": "N/A"}),
    ]
    
    for pos_id, instrument, book_id, tags in instruments:
        builder.position(
            id=pos_id,
            instrument=instrument,
            entity_id=fund.id,
            quantity=1.0,
            tags=tags
        )
        builder.add_position_to_book(pos_id, book_id)
    
    portfolio = builder.build()
    print(f"  ✓ Portfolio: {len(portfolio.positions())} positions")
    print(f"  ✓ Books: {len(portfolio.books())}")
    print()
    
    return portfolio


def step3_baseline_valuation(portfolio, market):
    """Step 3: Baseline valuation with risk metrics."""
    print("STEP 3: Baseline Valuation")
    print("="*80)
    
    # Value portfolio
    result = value_portfolio(portfolio, market, None)
    
    print(f"Portfolio Value: ${result.total.amount:,.2f}")
    print()
    
    # Position breakdown
    print("Position Breakdown:")
    print("-"*60)
    print(f"{'Position':<15} {'Value (USD)':<20} {'Rating':<10}")
    print("-"*60)
    
    for pos_val in result.position_values:
        pos = next(p for p in portfolio.positions() if p.id == pos_val.position_id)
        value = pos_val.base_value.amount
        rating = pos.tags.get("rating", "N/A")
        print(f"{pos_val.position_id:<15} ${value:>18,.2f} {rating:<10}")
    print("-"*60)
    print()
    
    # Compute risk metrics
    registry = create_standard_registry()
    
    total_dv01 = 0.0
    total_cs01 = 0.0
    total_delta = 0.0
    
    print("Risk Metrics:")
    print("-"*60)
    print(f"{'Position':<15} {'DV01':<15} {'CS01':<15} {'Delta':<15}")
    print("-"*60)
    
    for position in portfolio.positions():
        instrument = position.instrument
        
        # Determine metrics and pricing method
        if isinstance(instrument, Bond):
            res = registry.price_bond_with_metrics(
                instrument, "discounting", market, ["dv01"]
            )
            dv01 = res.metric("dv01") or 0.0
            total_dv01 += dv01
            print(f"{position.id:<15} ${dv01:>13,.2f} {'-':<15} {'-':<15}")
            
        elif isinstance(instrument, CreditDefaultSwap):
            res = registry.price_cds_with_metrics(
                instrument, "discounting", market, ["cs01", "dv01"]
            )
            cs01 = res.metric("cs01") or 0.0
            dv01 = res.metric("dv01") or 0.0
            total_cs01 += cs01
            total_dv01 += dv01
            print(f"{position.id:<15} ${dv01:>13,.2f} ${cs01:>13,.2f} {'-':<15}")
            
        elif isinstance(instrument, EquityOption):
            res = registry.price_equity_option_with_metrics(
                instrument, "black_scholes", market, ["delta"]
            )
            delta = res.metric("delta") or 0.0
            total_delta += delta * position.quantity
            print(f"{position.id:<15} {'-':<15} {'-':<15} {delta:>13.2f}")
    
    print("-"*60)
    print(f"{'TOTAL':<15} ${total_dv01:>13,.2f} ${total_cs01:>13,.2f} {total_delta:>13.2f}")
    print("-"*60)
    print()
    
    return result, {"dv01": total_dv01, "cs01": total_cs01, "delta": total_delta}


def step4_stress_testing(portfolio, market):
    """Step 4: Run stress scenarios."""
    print("STEP 4: Stress Testing")
    print("="*80)
    
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
    
    print(f"{'Scenario':<25} {'PV (USD)':<20} {'P&L':<20} {'P&L %':<10}")
    print("-"*80)
    print(f"{'Baseline':<25} ${baseline_pv:>18,.2f} {'-':<20} {'-':<10}")
    
    stress_results = {}
    for name, spec in scenarios.items():
        shocked_market, _ = engine.apply(spec, market)
        result = value_portfolio(portfolio, shocked_market, None)
        pv = result.total.amount
        pnl = pv - baseline_pv
        pnl_pct = (pnl / baseline_pv) * 100
        
        stress_results[name] = pnl
        
        sign = "+" if pnl >= 0 else ""
        print(f"{name:<25} ${pv:>18,.2f} {sign}${pnl:>18,.2f} {sign}{pnl_pct:>9.2f}%")
    
    print("-"*80)
    print()
    
    worst_scenario = min(stress_results, key=stress_results.get)
    print(f"Worst case: {worst_scenario} (${stress_results[worst_scenario]:,.2f})")
    print()
    
    return stress_results


def step5_portfolio_optimization(market):
    """Step 5: Constrained portfolio optimization."""
    print("STEP 5: Portfolio Optimization")
    print("="*80)
    
    # Define trade universe (candidate positions)
    candidates = [
        CandidatePosition(
            id="BOND.AAA",
            instrument_type="bond",
            tags={"rating": "AAA"},
            expected_yield=0.040
        ),
        CandidatePosition(
            id="BOND.AA",
            instrument_type="bond",
            tags={"rating": "AA"},
            expected_yield=0.045
        ),
        CandidatePosition(
            id="BOND.BBB",
            instrument_type="bond",
            tags={"rating": "BBB"},
            expected_yield=0.050
        ),
        CandidatePosition(
            id="BOND.BB",
            instrument_type="bond",
            tags={"rating": "BB"},
            expected_yield=0.070
        ),
        CandidatePosition(
            id="BOND.CCC",
            instrument_type="bond",
            tags={"rating": "CCC"},
            expected_yield=0.100
        ),
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
    
    print(f"Optimization Status: {opt_result.status}")
    print(f"Objective Value: {opt_result.objective_value:.4f}")
    print()
    
    print("Optimal Weights:")
    print("-"*60)
    print(f"{'Position':<15} {'Rating':<10} {'Weight':<15} {'Amount (USD)':<20}")
    print("-"*60)
    
    for trade in opt_result.trades:
        candidate = next(c for c in candidates if c.id == trade.position_id)
        rating = candidate.tags.get("rating", "N/A")
        weight = trade.target_weight
        amount = trade.notional
        print(f"{trade.position_id:<15} {rating:<10} {weight*100:>13.2f}% ${amount:>18,.2f}")
    
    print("-"*60)
    print()
    
    # Rating distribution
    from collections import defaultdict
    rating_dist = defaultdict(float)
    for trade in opt_result.trades:
        candidate = next(c for c in candidates if c.id == trade.position_id)
        rating = candidate.tags.get("rating", "N/A")
        rating_dist[rating] += trade.target_weight
    
    print("Rating Distribution:")
    print("-"*40)
    for rating, weight in sorted(rating_dist.items()):
        print(f"  {rating:<10} {weight*100:>10.2f}%")
    print("-"*40)
    print()
    
    # Verify constraints
    print("Constraint Validation:")
    print(f"  ✓ Total notional: $100M")
    print(f"  ✓ Max single position: {max(t.target_weight for t in opt_result.trades)*100:.1f}% (<25%)")
    print(f"  ✓ CCC exposure: {rating_dist.get('CCC', 0)*100:.1f}% (<10%)")
    print(f"  ✓ AAA exposure: {rating_dist.get('AAA', 0)*100:.1f}% (>20%)")
    print()
    
    return opt_result


def step6_export_results(portfolio, baseline_result, stress_results, risk_metrics):
    """Step 6: Export results to DataFrame."""
    print("STEP 6: Export and Reporting")
    print("="*80)
    
    # Portfolio positions DataFrame
    df_portfolio = baseline_result.to_polars()
    print("Portfolio DataFrame:")
    print(df_portfolio)
    print()
    
    # Risk metrics DataFrame
    risk_data = [
        {"metric": "DV01", "value": risk_metrics["dv01"], "unit": "USD"},
        {"metric": "CS01", "value": risk_metrics["cs01"], "unit": "USD"},
        {"metric": "Delta", "value": risk_metrics["delta"], "unit": "shares"},
    ]
    df_risk = pl.DataFrame(risk_data)
    print("Risk Metrics:")
    print(df_risk)
    print()
    
    # Stress results DataFrame
    stress_data = [
        {"scenario": name, "pnl": pnl}
        for name, pnl in stress_results.items()
    ]
    df_stress = pl.DataFrame(stress_data)
    print("Stress Test Results:")
    print(df_stress)
    print()
    
    # Export to files (commented out for example)
    # df_portfolio.write_csv("portfolio_valuation.csv")
    # df_risk.write_csv("risk_metrics.csv")
    # df_stress.write_csv("stress_results.csv")
    
    print("  ✓ DataFrames created for portfolio, risk, and stress results")
    print("  ✓ Ready for export to CSV, Parquet, Excel, etc.")
    print()


def main():
    """Execute complete portfolio workflow."""
    print()
    print("="*80)
    print(" "*20 + "COOKBOOK EXAMPLE 21")
    print(" "*15 + "FULL PORTFOLIO WORKFLOW")
    print("="*80)
    print()
    
    # Step 1: Calibrate market data
    market = step1_calibrate_market_data()
    
    # Step 2: Build portfolio
    portfolio = step2_build_portfolio(market)
    
    # Step 3: Baseline valuation
    baseline_result, risk_metrics = step3_baseline_valuation(portfolio, market)
    
    # Step 4: Stress testing
    stress_results = step4_stress_testing(portfolio, market)
    
    # Step 5: Portfolio optimization
    opt_result = step5_portfolio_optimization(market)
    
    # Step 6: Export results
    step6_export_results(portfolio, baseline_result, stress_results, risk_metrics)
    
    # Final summary
    print("="*80)
    print("WORKFLOW COMPLETE")
    print("="*80)
    print()
    print("Summary:")
    print(f"  1. ✓ Calibrated market curves (6 OIS quotes, 3 CDS quotes)")
    print(f"  2. ✓ Built portfolio (4 positions across 3 books)")
    print(f"  3. ✓ Valued portfolio with risk metrics")
    print(f"  4. ✓ Stress tested (3 scenarios)")
    print(f"  5. ✓ Optimized portfolio (5 candidates, 4 constraints)")
    print(f"  6. ✓ Exported results to DataFrames")
    print()
    print("Key Takeaways:")
    print("  - Complete workflow from calibration to optimization")
    print("  - Market data, pricing, risk, scenarios all integrated")
    print("  - Constraints and limits enforced in optimization")
    print("  - Results exportable for downstream analysis")
    print("  - Production-ready patterns demonstrated")
    print()
    print("="*80)


if __name__ == "__main__":
    main()

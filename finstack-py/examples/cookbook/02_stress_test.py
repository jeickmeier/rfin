"""
Title: Scenario-Based Stress Testing
Persona: Portfolio Manager, Risk Analyst
Complexity: Intermediate
Runtime: ~3 seconds

Description:
Demonstrates comprehensive stress testing workflow:
- Build base portfolio with mixed instruments
- Define multiple stress scenarios (rates, credit, FX, equity)
- Apply scenarios using ScenarioEngine
- Compare before/after valuations
- Analyze P&L impact by scenario

Key Concepts:
- Scenario construction with builder API
- Market data shocks (parallel and bucketed)
- Scenario composition and comparison
- Impact analysis and reporting

Prerequisites:
- Portfolio construction (Example 01)
- Basic scenario concepts
"""

from finstack import (
    PortfolioBuilder,
    Entity,
    Bond,
    CreditDefaultSwap,
    EquityOption,
    Money,
    Date,
    MarketContext,
    DiscountCurve,
    HazardCurve,
    FxMatrix,
    create_standard_registry,
    value_portfolio,
)
from finstack.scenarios import (
    ScenarioEngine,
    scenario,  # Builder API
)


def create_base_market():
    """Create baseline market data."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))
    
    # Discount curves
    usd_curve = DiscountCurve.flat(
        id="USD.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.045,
        day_count="Act360"
    )
    market.insert_discount(usd_curve)
    
    # Credit curves (for CDS)
    acme_hazard = HazardCurve.flat(
        id="ACME.5Y",
        base_date=Date(2024, 1, 15),
        rate=0.015,  # 150bps credit spread
        recovery_rate=0.40
    )
    market.insert_hazard(acme_hazard)
    
    # FX rates
    fx = FxMatrix()
    fx.set_spot("USD", "EUR", 0.92)
    market.set_fx_matrix(fx)
    
    # Equity spot prices
    market.set_equity("SPY", 480.0)  # S&P 500 ETF
    
    return market


def create_portfolio():
    """Create test portfolio with mixed instruments."""
    builder = PortfolioBuilder()
    builder.base_currency("USD")
    builder.as_of(Date(2024, 1, 15))
    
    # Add entity
    fund = Entity(
        id="FUND-001",
        name="Multi-Strategy Fund",
        tags={"strategy": "multi-asset"}
    )
    builder.entity(fund)
    
    # 1. Corporate bonds (interest rate sensitive)
    bond1 = Bond.fixed_semiannual(
        id="CORP.5Y",
        notional=Money.from_code(10_000_000, "USD"),
        coupon_rate=0.05,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-BOND-1",
        instrument=bond1,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "rates"}
    )
    
    # 2. CDS (credit sensitive)
    cds = CreditDefaultSwap(
        id="ACME.CDS.5Y",
        notional=Money.from_code(5_000_000, "USD"),
        spread=0.0150,  # 150bps running spread
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),
        is_protection_buyer=True,  # Long protection
        hazard_curve_id="ACME.5Y",
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-CDS-1",
        instrument=cds,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "credit"}
    )
    
    # 3. Equity option (equity/vol sensitive)
    eq_call = EquityOption.european(
        id="SPY.CALL.6M",
        strike=500.0,
        expiry=Date(2024, 7, 15),
        is_call=True,
        underlying="SPY",
        quantity=100.0,  # 100 shares per option
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-EQ-CALL",
        instrument=eq_call,
        entity_id=fund.id,
        quantity=10.0,  # 10 contracts
        tags={"asset_class": "equity"}
    )
    
    return builder.build()


def define_scenarios():
    """Define stress scenarios."""
    scenarios = {}
    
    # 1. Rate shock: +100bp parallel shift
    scenarios["RATE_UP_100BP"] = (
        scenario("Rate Shock +100bp")
        .description("Parallel +100bp shift to all discount curves")
        .shift_discount_curve("USD.OIS", 100)  # +100bp
        .build()
    )
    
    # 2. Credit widening: +200bp to credit spreads
    scenarios["CREDIT_WIDEN_200BP"] = (
        scenario("Credit Widening +200bp")
        .description("Parallel +200bp shift to credit curves")
        .shift_hazard_curve("ACME.5Y", 200)  # +200bp
        .build()
    )
    
    # 3. Equity crash: -20%
    scenarios["EQUITY_CRASH_20PCT"] = (
        scenario("Equity Crash -20%")
        .description("Equity market crash scenario")
        .shift_equity("SPY", -20.0)  # -20%
        .build()
    )
    
    # 4. FX shock: EUR strengthens 10%
    scenarios["FX_EUR_STRONG_10PCT"] = (
        scenario("EUR Strengthens +10%")
        .description("EUR/USD moves from 0.92 to 1.012 (10% EUR appreciation)")
        .shift_fx("USD", "EUR", 10.0)  # +10%
        .build()
    )
    
    # 5. Combined stress: Rates up, credit wide, equity down
    scenarios["COMBINED_STRESS"] = (
        scenario("Combined Stress")
        .description("Multi-factor stress: rates +50bp, credit +100bp, equity -10%")
        .shift_discount_curve("USD.OIS", 50)
        .shift_hazard_curve("ACME.5Y", 100)
        .shift_equity("SPY", -10.0)
        .build()
    )
    
    return scenarios


def main():
    """Run stress testing workflow."""
    print("="*80)
    print("COOKBOOK EXAMPLE 02: Scenario-Based Stress Testing")
    print("="*80)
    print()
    
    # 1. Create base market and portfolio
    print("1. Creating base market and portfolio...")
    market = create_base_market()
    portfolio = create_portfolio()
    print(f"  ✓ Portfolio: {len(portfolio.positions())} positions")
    print(f"  ✓ Market as-of: {market.as_of()}")
    print()
    
    # 2. Value at baseline
    print("2. Baseline valuation...")
    baseline_result = value_portfolio(portfolio, market, None)
    baseline_pv = baseline_result.total.amount
    print(f"  ✓ Baseline portfolio value: ${baseline_pv:,.2f}")
    print()
    
    print("  Position breakdown:")
    print("  " + "-"*76)
    print(f"  {'Position':<20} {'Asset Class':<15} {'Value (USD)':<20}")
    print("  " + "-"*76)
    for pos_val in baseline_result.position_values:
        pos_id = pos_val.position_id
        pos = next(p for p in portfolio.positions() if p.id == pos_id)
        asset_class = pos.tags.get("asset_class", "N/A")
        value = pos_val.base_value.amount
        print(f"  {pos_id:<20} {asset_class:<15} ${value:>18,.2f}")
    print("  " + "-"*76)
    print()
    
    # 3. Define scenarios
    print("3. Defining stress scenarios...")
    scenarios = define_scenarios()
    print(f"  ✓ Created {len(scenarios)} scenarios:")
    for name, spec in scenarios.items():
        print(f"    - {name}: {spec.description}")
    print()
    
    # 4. Run stress tests
    print("4. Running stress tests...")
    print("="*80)
    
    engine = ScenarioEngine()
    results = {}
    
    for scenario_name, scenario_spec in scenarios.items():
        print(f"\nScenario: {scenario_name}")
        print("-"*80)
        
        # Apply scenario to market
        shocked_market, report = engine.apply(scenario_spec, market)
        
        # Revalue portfolio
        stressed_result = value_portfolio(portfolio, shocked_market, None)
        stressed_pv = stressed_result.total.amount
        
        # Calculate impact
        pnl = stressed_pv - baseline_pv
        pnl_pct = (pnl / baseline_pv) * 100
        
        results[scenario_name] = {
            "pv": stressed_pv,
            "pnl": pnl,
            "pnl_pct": pnl_pct,
            "result": stressed_result
        }
        
        print(f"Baseline PV:     ${baseline_pv:>15,.2f}")
        print(f"Stressed PV:     ${stressed_pv:>15,.2f}")
        print(f"P&L Impact:      ${pnl:>15,.2f}")
        print(f"P&L %:           {pnl_pct:>15.2f}%")
        print()
        
        # Position-level impact
        print("Position-level impact:")
        print(f"{'Position':<20} {'Baseline':<18} {'Stressed':<18} {'P&L':<18}")
        print("-"*76)
        
        for baseline_pos in baseline_result.position_values:
            pos_id = baseline_pos.position_id
            baseline_val = baseline_pos.base_value.amount
            
            stressed_pos = next(
                p for p in stressed_result.position_values
                if p.position_id == pos_id
            )
            stressed_val = stressed_pos.base_value.amount
            
            pos_pnl = stressed_val - baseline_val
            
            print(f"{pos_id:<20} "
                  f"${baseline_val:>16,.2f} "
                  f"${stressed_val:>16,.2f} "
                  f"${pos_pnl:>16,.2f}")
        print("-"*76)
    
    print()
    
    # 5. Summary comparison
    print("5. Stress Test Summary")
    print("="*80)
    print(f"{'Scenario':<30} {'P&L Impact':<20} {'P&L %':<15}")
    print("-"*80)
    for scenario_name in scenarios.keys():
        pnl = results[scenario_name]["pnl"]
        pnl_pct = results[scenario_name]["pnl_pct"]
        
        # Color code (if terminal supports it)
        sign = "+" if pnl >= 0 else ""
        print(f"{scenario_name:<30} "
              f"{sign}${pnl:>18,.2f} "
              f"{sign}{pnl_pct:>14.2f}%")
    print("-"*80)
    print()
    
    # 6. Worst case scenario
    worst_scenario = min(results.items(), key=lambda x: x[1]["pnl"])
    print(f"Worst Case Scenario: {worst_scenario[0]}")
    print(f"  P&L Impact: ${worst_scenario[1]['pnl']:,.2f} ({worst_scenario[1]['pnl_pct']:.2f}%)")
    print()
    
    # Best case scenario
    best_scenario = max(results.items(), key=lambda x: x[1]["pnl"])
    print(f"Best Case Scenario: {best_scenario[0]}")
    print(f"  P&L Impact: ${best_scenario[1]['pnl']:,.2f} ({best_scenario[1]['pnl_pct']:.2f}%)")
    print()
    
    # 7. Export to DataFrame for further analysis
    print("7. Export Results")
    print("="*80)
    
    # Create comparison DataFrame
    import polars as pl
    
    scenario_data = []
    for scenario_name, res in results.items():
        scenario_data.append({
            "scenario": scenario_name,
            "baseline_pv": baseline_pv,
            "stressed_pv": res["pv"],
            "pnl": res["pnl"],
            "pnl_pct": res["pnl_pct"]
        })
    
    df = pl.DataFrame(scenario_data)
    print(df)
    print()
    
    # Can export to CSV/Parquet
    # df.write_csv("stress_test_results.csv")
    # df.write_parquet("stress_test_results.parquet")
    
    print("="*80)
    print("EXAMPLE COMPLETE")
    print("="*80)
    print()
    print("Key Takeaways:")
    print("- Scenarios can shock rates, credit, equity, FX independently or combined")
    print("- ScenarioEngine applies shocks deterministically to market data")
    print("- Portfolio revaluation shows position-level and aggregate impact")
    print("- Results can be exported to DataFrame for visualization/reporting")
    print("- Worst-case scenario analysis helps identify risk concentrations")


if __name__ == "__main__":
    main()

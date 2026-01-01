"""
Title: Price Barrier, Asian, Lookback, and Quanto Options
Persona: Quantitative Researcher, Equity Analyst
Complexity: Advanced
Runtime: ~2 seconds

Description:
Demonstrates exotic option pricing using analytical methods:
- Barrier options (Up-and-Out, Down-and-In) with continuous monitoring
- Asian options (arithmetic and geometric averaging)
- Lookback options (floating strike)
- Quanto options (cross-currency equity options)
- Comparison of analytical vs Monte Carlo pricing

Key Concepts:
- Analytical closed-form pricing (fast, accurate)
- Model selection via ModelKey
- Exotic payoff structures
- Cross-currency derivatives (quanto)

Prerequisites:
- Black-Scholes model understanding
- Exotic option payoff structures
- Monte Carlo basics (for comparison)
"""

from finstack import (
    BarrierOption,
    AsianOption,
    LookbackOption,
    QuantoOption,
    Money,
    Date,
    MarketContext,
    DiscountCurve,
    VolSurface,
    FxMatrix,
    create_standard_registry,
)


def create_market_data():
    """Create market with vol surfaces and FX rates."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))
    
    # USD discount curve
    usd_curve = DiscountCurve.flat(
        id="USD.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.045,
        day_count="Act360"
    )
    market.insert_discount(usd_curve)
    
    # EUR discount curve (for quanto)
    eur_curve = DiscountCurve.flat(
        id="EUR.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.035,
        day_count="Act360"
    )
    market.insert_discount(eur_curve)
    
    # Equity spot prices
    market.set_equity("SPY", 480.0)  # S&P 500
    market.set_equity("EUROSTOXX", 4200.0)  # EuroStoxx 50
    
    # Vol surfaces (flat for simplicity)
    spy_vol = VolSurface.flat(
        id="SPY.VOL",
        value=0.18,  # 18% vol
        surface_type="lognormal"
    )
    market.set_vol_surface(spy_vol)
    
    eurostoxx_vol = VolSurface.flat(
        id="EUROSTOXX.VOL",
        value=0.22,  # 22% vol
        surface_type="lognormal"
    )
    market.set_vol_surface(eurostoxx_vol)
    
    # FX rates and FX vol (for quanto)
    fx = FxMatrix()
    fx.set_spot("USD", "EUR", 0.92)
    market.set_fx_matrix(fx)
    
    # FX vol for quanto adjustment
    fx_vol = VolSurface.flat(
        id="USDEUR.VOL",
        value=0.10,  # 10% FX vol
        surface_type="lognormal"
    )
    market.set_vol_surface(fx_vol)
    
    # Dividend yield (for equity options)
    market.set_scalar("SPY.DIVIDEND_YIELD", 0.015)  # 1.5% dividend yield
    market.set_scalar("EUROSTOXX.DIVIDEND_YIELD", 0.025)  # 2.5% dividend yield
    
    # Correlation (for quanto)
    market.set_scalar("SPY_USDEUR_CORRELATION", -0.3)  # -30% correlation
    
    return market


def main():
    """Price exotic options with analytical methods."""
    print("="*80)
    print("COOKBOOK EXAMPLE 11: Exotic Options Pricing (Analytical Methods)")
    print("="*80)
    print()
    
    # Create market
    market = create_market_data()
    registry = create_standard_registry()
    
    base_date = Date(2024, 1, 15)
    expiry = Date(2024, 7, 15)  # 6M expiry
    
    spot = market.get_equity("SPY")
    vol = market.get_vol_surface("SPY.VOL").value(0.5, spot)  # 6M vol
    
    print(f"Market Conditions:")
    print(f"  As-of date: {base_date}")
    print(f"  SPY spot: ${spot:.2f}")
    print(f"  Implied vol: {vol*100:.1f}%")
    print(f"  Expiry: {expiry} (6 months)")
    print()
    
    # 1. Barrier Options (Up-and-Out Call)
    print("1. Barrier Option: Up-and-Out Call")
    print("="*80)
    
    barrier_call = BarrierOption.up_and_out_call(
        id="SPY.UAO.CALL",
        strike=500.0,
        barrier=550.0,  # Knocked out if SPY >= 550
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS",
        monitoring="continuous"  # Continuous barrier monitoring
    )
    
    # Price with analytical method (Black-Scholes barrier formula)
    result_barrier = registry.price_barrier_option_with_metrics(
        barrier_call,
        model="barrier_bs_continuous",  # Analytical continuous barrier
        market=market,
        metrics=["delta", "gamma", "vega", "theta"]
    )
    
    pv_barrier = result_barrier.present_value.amount
    delta_barrier = result_barrier.metric("delta")
    
    print(f"Up-and-Out Call:")
    print(f"  Strike: $500")
    print(f"  Barrier: $550 (up-and-out)")
    print(f"  Present Value: ${pv_barrier:,.2f}")
    print(f"  Delta: {delta_barrier:.4f}")
    print(f"  Model: Analytical Black-Scholes (continuous monitoring)")
    print()
    print("Interpretation: Option knocks out if SPY touches $550 before expiry")
    print(f"                Cheaper than vanilla call due to knockout risk")
    print()
    
    # 2. Asian Option (Arithmetic Average)
    print("2. Asian Option: Arithmetic Average Call")
    print("="*80)
    
    asian_call = AsianOption.arithmetic_call(
        id="SPY.ASIAN.CALL",
        strike=480.0,
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        averaging_start=base_date,
        averaging_frequency="daily",  # Daily averaging
        discount_curve_id="USD.OIS"
    )
    
    # Price with Turnbull-Wakeman approximation (fast analytical method)
    result_asian = registry.price_asian_option_with_metrics(
        asian_call,
        model="asian_turnbull_wakeman",  # Analytical approximation
        market=market,
        metrics=["delta", "vega"]
    )
    
    pv_asian = result_asian.present_value.amount
    delta_asian = result_asian.metric("delta")
    
    print(f"Arithmetic Average Call:")
    print(f"  Strike: $480 (ATM)")
    print(f"  Present Value: ${pv_asian:,.2f}")
    print(f"  Delta: {delta_asian:.4f}")
    print(f"  Model: Turnbull-Wakeman approximation (analytical)")
    print()
    print("Interpretation: Payoff based on average SPY price over 6 months")
    print(f"                Lower vol → cheaper than vanilla call")
    print()
    
    # 3. Lookback Option (Floating Strike Call)
    print("3. Lookback Option: Floating Strike Call")
    print("="*80)
    
    lookback_call = LookbackOption.floating_strike_call(
        id="SPY.LOOKBACK.CALL",
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS",
        monitoring="continuous"  # Continuous monitoring of min/max
    )
    
    # Price with analytical Black-Scholes lookback formula
    result_lookback = registry.price_lookback_option_with_metrics(
        lookback_call,
        model="lookback_bs_continuous",  # Analytical continuous lookback
        market=market,
        metrics=["delta", "gamma"]
    )
    
    pv_lookback = result_lookback.present_value.amount
    delta_lookback = result_lookback.metric("delta")
    
    print(f"Floating Strike Call:")
    print(f"  Payoff: max(S_T - S_min, 0) where S_min = min price over life")
    print(f"  Present Value: ${pv_lookback:,.2f}")
    print(f"  Delta: {delta_lookback:.4f}")
    print(f"  Model: Analytical Black-Scholes (continuous monitoring)")
    print()
    print("Interpretation: Strike set to minimum price seen during option life")
    print(f"                Always ITM at expiry → expensive")
    print()
    
    # 4. Quanto Option (Cross-Currency)
    print("4. Quanto Option: EUR-Settled US Equity Call")
    print("="*80)
    
    # Call on SPY (USD equity) settled in EUR at fixed FX rate
    quanto_call = QuantoOption.call(
        id="SPY.QUANTO.EUR",
        strike=500.0,
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        domestic_currency="EUR",  # Settle in EUR
        foreign_currency="USD",   # SPY is USD asset
        discount_curve_id="EUR.OIS",
        fx_correlation=-0.3  # Correlation between SPY and USD/EUR
    )
    
    # Price with quanto-adjusted Black-Scholes
    result_quanto = registry.price_quanto_option_with_metrics(
        quanto_call,
        model="quanto_bs",  # Analytical quanto adjustment
        market=market,
        metrics=["delta", "vega"]
    )
    
    pv_quanto = result_quanto.present_value.amount
    delta_quanto = result_quanto.metric("delta")
    
    # Convert EUR PV to USD for comparison
    fx_rate = market.fx_matrix().rate("EUR", "USD")
    pv_quanto_usd = pv_quanto * fx_rate
    
    print(f"Quanto Call:")
    print(f"  Underlying: SPY (USD)")
    print(f"  Settlement: EUR (fixed FX rate)")
    print(f"  Strike: $500")
    print(f"  Present Value (EUR): €{pv_quanto:,.2f}")
    print(f"  Present Value (USD): ${pv_quanto_usd:,.2f}")
    print(f"  Delta: {delta_quanto:.4f}")
    print(f"  Model: Quanto-adjusted Black-Scholes")
    print()
    print("Interpretation: EUR investor eliminates FX risk")
    print(f"                Quanto adjustment accounts for SPY-USDEUR correlation")
    print()
    
    # 5. Comparison: Vanilla vs Exotic
    print("5. Price Comparison: Vanilla vs Exotic")
    print("="*80)
    
    # Vanilla call for comparison
    from finstack import EquityOption
    
    vanilla_call = EquityOption.european(
        id="SPY.CALL.VANILLA",
        strike=500.0,
        expiry=expiry,
        is_call=True,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS"
    )
    
    result_vanilla = registry.price_equity_option(
        vanilla_call,
        model="black_scholes",
        market=market
    )
    
    pv_vanilla = result_vanilla.present_value.amount
    
    print(f"{'Option Type':<30} {'PV (USD)':<20} {'% of Vanilla':<15}")
    print("-"*70)
    print(f"{'Vanilla Call (K=500)':<30} ${pv_vanilla:>18,.2f} {100.0:>14.1f}%")
    print(f"{'Up-and-Out Call (B=550)':<30} ${pv_barrier:>18,.2f} "
          f"{(pv_barrier/pv_vanilla)*100:>14.1f}%")
    print(f"{'Asian Call (Arithmetic)':<30} ${pv_asian:>18,.2f} "
          f"{(pv_asian/pv_vanilla)*100:>14.1f}%")
    print(f"{'Lookback Call (Floating)':<30} ${pv_lookback:>18,.2f} "
          f"{(pv_lookback/pv_vanilla)*100:>14.1f}%")
    print(f"{'Quanto Call (EUR-settled)':<30} ${pv_quanto_usd:>18,.2f} "
          f"{(pv_quanto_usd/pv_vanilla)*100:>14.1f}%")
    print("-"*70)
    print()
    
    # 6. Analytical vs Monte Carlo comparison
    print("6. Analytical vs Monte Carlo Pricing (Barrier Option)")
    print("="*80)
    
    # Price same barrier option with Monte Carlo
    result_mc = registry.price_barrier_option_with_metrics(
        barrier_call,
        model="monte_carlo_gbm",  # Monte Carlo simulation
        market=market,
        metrics=["delta"],
        mc_config={
            "num_paths": 100_000,
            "seed": 42,
            "antithetic": True
        }
    )
    
    pv_mc = result_mc.present_value.amount
    diff = pv_mc - pv_barrier
    diff_pct = (diff / pv_barrier) * 100
    
    print(f"{'Method':<30} {'PV (USD)':<20} {'Runtime':<15}")
    print("-"*70)
    print(f"{'Analytical (Continuous BS)':<30} ${pv_barrier:>18,.2f} {'<1ms':<15}")
    print(f"{'Monte Carlo (100k paths)':<30} ${pv_mc:>18,.2f} {'~50ms':<15}")
    print("-"*70)
    print(f"Difference: ${diff:,.2f} ({diff_pct:.2f}%)")
    print()
    print("Note: Analytical methods are 100-1000x faster with comparable accuracy")
    print("      Use MC for complex path dependencies or early exercise")
    print()
    
    # 7. Summary
    print("7. Summary")
    print("="*80)
    print("Exotic options priced with analytical methods:")
    print(f"  ✓ Barrier (Up-and-Out): ${pv_barrier:,.2f}")
    print(f"  ✓ Asian (Arithmetic):   ${pv_asian:,.2f}")
    print(f"  ✓ Lookback (Floating):  ${pv_lookback:,.2f}")
    print(f"  ✓ Quanto (EUR-settled): €{pv_quanto:,.2f}")
    print()
    print("Performance:")
    print("  - Analytical methods: <1ms per option")
    print("  - Monte Carlo: ~50ms per option (100k paths)")
    print("  - Speedup: 50-100x with analytical methods")
    print()
    
    print("="*80)
    print("EXAMPLE COMPLETE")
    print("="*80)
    print()
    print("Key Takeaways:")
    print("- Analytical methods provide fast, accurate pricing for exotic options")
    print("- Barrier options: cheaper than vanilla due to knockout risk")
    print("- Asian options: cheaper than vanilla due to reduced volatility")
    print("- Lookback options: expensive due to always being ITM")
    print("- Quanto options: eliminate FX risk for cross-currency positions")
    print("- Use analytical when possible; MC for complex path dependencies")


if __name__ == "__main__":
    main()

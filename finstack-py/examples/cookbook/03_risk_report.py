"""
Title: Comprehensive Risk Report (DV01/CS01/Greeks)
Persona: Risk Analyst
Complexity: Intermediate
Runtime: ~4 seconds

Description:
Generates comprehensive risk report with:
- DV01 (interest rate sensitivity per $1M notional)
- CS01 (credit spread sensitivity per $1M notional)
- Options Greeks (Delta, Gamma, Vega, Theta, Rho)
- Risk aggregation by asset class and currency
- Risk ladder by tenor/maturity

Key Concepts:
- Metrics computation via price_with_metrics
- Risk aggregation across positions
- Greeks for options
- Risk bucketing by maturity

Prerequisites:
- Portfolio construction (Example 01)
- Understanding of risk metrics (DV01, Greeks)
"""

from finstack import (
    PortfolioBuilder,
    Entity,
    Bond,
    CreditDefaultSwap,
    EquityOption,
    InterestRateSwap,
    Money,
    Date,
    MarketContext,
    DiscountCurve,
    HazardCurve,
    VolSurface,
    FxMatrix,
    create_standard_registry,
    value_portfolio,
)


def create_market_data():
    """Create market with curves and vol surfaces."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))
    
    # Discount curve
    usd_curve = DiscountCurve.flat(
        id="USD.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.045,
        day_count="Act360"
    )
    market.insert_discount(usd_curve)
    
    # Hazard curve for CDS
    corp_hazard = HazardCurve.flat(
        id="CORP.CDS",
        base_date=Date(2024, 1, 15),
        rate=0.020,  # 200bps spread
        recovery_rate=0.40
    )
    market.insert_hazard(corp_hazard)
    
    # Vol surface for equity options
    vol_surface = VolSurface.flat(
        id="SPY.VOL",
        value=0.18,  # 18% flat vol
        surface_type="lognormal"
    )
    market.set_vol_surface(vol_surface)
    
    # Equity spot
    market.set_equity("SPY", 480.0)
    
    # FX
    fx = FxMatrix()
    fx.set_spot("USD", "EUR", 0.92)
    market.set_fx_matrix(fx)
    
    return market


def create_diversified_portfolio():
    """Create portfolio with diverse risk exposures."""
    builder = PortfolioBuilder()
    builder.base_currency("USD")
    builder.as_of(Date(2024, 1, 15))
    
    fund = Entity(id="FUND-001", name="Global Macro Fund")
    builder.entity(fund)
    
    # 1. Short-term bond (low duration)
    bond_2y = Bond.fixed_semiannual(
        id="BOND.2Y",
        notional=Money.from_code(10_000_000, "USD"),
        coupon_rate=0.04,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2026, 1, 15),  # 2Y
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-BOND-2Y",
        instrument=bond_2y,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "rates", "maturity_bucket": "0-2Y"}
    )
    
    # 2. Medium-term bond
    bond_5y = Bond.fixed_semiannual(
        id="BOND.5Y",
        notional=Money.from_code(20_000_000, "USD"),
        coupon_rate=0.045,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),  # 5Y
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-BOND-5Y",
        instrument=bond_5y,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "rates", "maturity_bucket": "5-7Y"}
    )
    
    # 3. Long-term bond (high duration)
    bond_10y = Bond.fixed_semiannual(
        id="BOND.10Y",
        notional=Money.from_code(15_000_000, "USD"),
        coupon_rate=0.05,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2034, 1, 15),  # 10Y
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-BOND-10Y",
        instrument=bond_10y,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "rates", "maturity_bucket": "10Y+"}
    )
    
    # 4. Interest rate swap (receiver)
    irs = InterestRateSwap.fixed_vs_float(
        id="IRS.7Y",
        notional=Money.from_code(25_000_000, "USD"),
        fixed_rate=0.045,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2031, 1, 15),  # 7Y
        pay_fixed=False,  # Receiver
        discount_curve_id="USD.OIS",
        forward_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-IRS-7Y",
        instrument=irs,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "rates", "maturity_bucket": "5-7Y"}
    )
    
    # 5. CDS (long protection)
    cds = CreditDefaultSwap(
        id="CDS.5Y",
        notional=Money.from_code(10_000_000, "USD"),
        spread=0.020,  # 200bps
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),  # 5Y
        is_protection_buyer=True,
        hazard_curve_id="CORP.CDS",
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-CDS-5Y",
        instrument=cds,
        entity_id=fund.id,
        quantity=1.0,
        tags={"asset_class": "credit", "maturity_bucket": "5-7Y"}
    )
    
    # 6. Equity call options (multiple strikes for gamma/vega)
    for i, strike in enumerate([460, 480, 500], 1):
        option = EquityOption.european(
            id=f"SPY.CALL.{strike}",
            strike=float(strike),
            expiry=Date(2024, 7, 15),  # 6M
            is_call=True,
            underlying="SPY",
            quantity=100.0,
            discount_curve_id="USD.OIS"
        )
        builder.position(
            id=f"POS-CALL-{strike}",
            instrument=option,
            entity_id=fund.id,
            quantity=10.0,
            tags={"asset_class": "equity", "maturity_bucket": "0-1Y"}
        )
    
    # 7. Equity put option (for delta hedging)
    put = EquityOption.european(
        id="SPY.PUT.460",
        strike=460.0,
        expiry=Date(2024, 7, 15),
        is_call=False,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS"
    )
    builder.position(
        id="POS-PUT-460",
        instrument=put,
        entity_id=fund.id,
        quantity=20.0,
        tags={"asset_class": "equity", "maturity_bucket": "0-1Y"}
    )
    
    return builder.build()


def compute_risk_metrics(portfolio, market):
    """Compute risk metrics for all positions."""
    registry = create_standard_registry()
    risk_data = []
    
    print("Computing metrics for each position...")
    for position in portfolio.positions():
        pos_id = position.id
        instrument = position.instrument
        
        # Define metrics based on instrument type
        if isinstance(instrument, (Bond, InterestRateSwap)):
            metrics = ["dv01", "duration_mod", "convexity"]
        elif isinstance(instrument, CreditDefaultSwap):
            metrics = ["cs01", "dv01"]
        elif isinstance(instrument, EquityOption):
            metrics = ["delta", "gamma", "vega", "theta", "rho"]
        else:
            metrics = []
        
        if not metrics:
            continue
        
        # Price with metrics
        try:
            if isinstance(instrument, Bond):
                result = registry.price_bond_with_metrics(
                    instrument, "discounting", market, metrics
                )
            elif isinstance(instrument, InterestRateSwap):
                result = registry.price_swap_with_metrics(
                    instrument, "discounting", market, metrics
                )
            elif isinstance(instrument, CreditDefaultSwap):
                result = registry.price_cds_with_metrics(
                    instrument, "discounting", market, metrics
                )
            elif isinstance(instrument, EquityOption):
                result = registry.price_equity_option_with_metrics(
                    instrument, "black_scholes", market, metrics
                )
            else:
                continue
            
            # Extract metrics
            metrics_dict = {
                "position_id": pos_id,
                "asset_class": position.tags.get("asset_class", "N/A"),
                "maturity_bucket": position.tags.get("maturity_bucket", "N/A"),
                "pv": result.present_value.amount,
            }
            
            for metric in metrics:
                value = result.metric(metric)
                if value is not None:
                    metrics_dict[metric] = value
            
            risk_data.append(metrics_dict)
            
        except Exception as e:
            print(f"  Warning: Could not compute metrics for {pos_id}: {e}")
    
    return risk_data


def main():
    """Generate comprehensive risk report."""
    print("="*80)
    print("COOKBOOK EXAMPLE 03: Comprehensive Risk Report")
    print("="*80)
    print()
    
    # 1. Create market and portfolio
    print("1. Creating market data and portfolio...")
    market = create_market_data()
    portfolio = create_diversified_portfolio()
    print(f"  ✓ Portfolio: {len(portfolio.positions())} positions")
    print()
    
    # 2. Compute risk metrics
    print("2. Computing risk metrics...")
    risk_data = compute_risk_metrics(portfolio, market)
    print(f"  ✓ Computed metrics for {len(risk_data)} positions")
    print()
    
    # 3. Interest Rate Risk (DV01)
    print("3. Interest Rate Risk")
    print("="*80)
    print(f"{'Position':<20} {'PV (USD)':<18} {'DV01':<15} {'Duration':<12} {'Convexity':<12}")
    print("-"*80)
    
    total_dv01 = 0.0
    for pos in risk_data:
        if "dv01" in pos:
            pos_id = pos["position_id"]
            pv = pos["pv"]
            dv01 = pos.get("dv01", 0.0)
            duration = pos.get("duration_mod", 0.0)
            convexity = pos.get("convexity", 0.0)
            
            total_dv01 += dv01
            
            print(f"{pos_id:<20} "
                  f"${pv:>16,.2f} "
                  f"${dv01:>13,.2f} "
                  f"{duration:>11.2f} "
                  f"{convexity:>11.2f}")
    
    print("-"*80)
    print(f"{'TOTAL DV01':<20} {'':<18} ${total_dv01:>13,.2f}")
    print("-"*80)
    print()
    print(f"Interpretation: Portfolio loses ${abs(total_dv01):,.2f} for 1bp parallel rate increase")
    print()
    
    # 4. Credit Risk (CS01)
    print("4. Credit Spread Risk")
    print("="*80)
    print(f"{'Position':<20} {'PV (USD)':<18} {'CS01':<15}")
    print("-"*80)
    
    total_cs01 = 0.0
    for pos in risk_data:
        if "cs01" in pos:
            pos_id = pos["position_id"]
            pv = pos["pv"]
            cs01 = pos.get("cs01", 0.0)
            
            total_cs01 += cs01
            
            print(f"{pos_id:<20} "
                  f"${pv:>16,.2f} "
                  f"${cs01:>13,.2f}")
    
    print("-"*80)
    print(f"{'TOTAL CS01':<20} {'':<18} ${total_cs01:>13,.2f}")
    print("-"*80)
    print()
    print(f"Interpretation: Portfolio loses ${abs(total_cs01):,.2f} for 1bp credit spread widening")
    print()
    
    # 5. Equity Options Greeks
    print("5. Equity Options Greeks")
    print("="*80)
    print(f"{'Position':<18} {'PV':<14} {'Delta':<12} {'Gamma':<12} {'Vega':<12} {'Theta':<12}")
    print("-"*80)
    
    total_delta = 0.0
    total_gamma = 0.0
    total_vega = 0.0
    total_theta = 0.0
    
    for pos in risk_data:
        if pos["asset_class"] == "equity":
            pos_id = pos["position_id"]
            pv = pos["pv"]
            delta = pos.get("delta", 0.0)
            gamma = pos.get("gamma", 0.0)
            vega = pos.get("vega", 0.0)
            theta = pos.get("theta", 0.0)
            
            total_delta += delta
            total_gamma += gamma
            total_vega += vega
            total_theta += theta
            
            print(f"{pos_id:<18} "
                  f"${pv:>12,.2f} "
                  f"{delta:>11.2f} "
                  f"{gamma:>11.4f} "
                  f"${vega:>10,.2f} "
                  f"${theta:>10,.2f}")
    
    print("-"*80)
    print(f"{'TOTAL':<18} {'':<14} "
          f"{total_delta:>11.2f} "
          f"{total_gamma:>11.4f} "
          f"${total_vega:>10,.2f} "
          f"${total_theta:>10,.2f}")
    print("-"*80)
    print()
    print("Interpretation:")
    print(f"  Delta: Portfolio exposure to $1 move in SPY = ${total_delta:,.2f}")
    print(f"  Gamma: Delta change for $1 move in SPY = {total_gamma:.4f}")
    print(f"  Vega: P&L for 1% vol increase = ${total_vega:,.2f}")
    print(f"  Theta: Daily time decay = ${total_theta:,.2f}")
    print()
    
    # 6. Risk Ladder by Maturity Bucket
    print("6. Risk Ladder by Maturity Bucket")
    print("="*80)
    
    # Aggregate DV01 by maturity bucket
    from collections import defaultdict
    dv01_by_bucket = defaultdict(float)
    pv_by_bucket = defaultdict(float)
    
    for pos in risk_data:
        if "dv01" in pos:
            bucket = pos["maturity_bucket"]
            dv01_by_bucket[bucket] += pos.get("dv01", 0.0)
            pv_by_bucket[bucket] += pos["pv"]
    
    print(f"{'Maturity Bucket':<20} {'PV (USD)':<18} {'DV01':<15}")
    print("-"*80)
    
    for bucket in sorted(dv01_by_bucket.keys()):
        pv = pv_by_bucket[bucket]
        dv01 = dv01_by_bucket[bucket]
        print(f"{bucket:<20} ${pv:>16,.2f} ${dv01:>13,.2f}")
    
    print("-"*80)
    print()
    
    # 7. Summary Dashboard
    print("7. Risk Dashboard Summary")
    print("="*80)
    print(f"Total Positions:     {len(portfolio.positions())}")
    print(f"Total PV:            ${sum(p['pv'] for p in risk_data):,.2f}")
    print()
    print("Interest Rate Risk:")
    print(f"  Total DV01:        ${total_dv01:,.2f}")
    print()
    print("Credit Risk:")
    print(f"  Total CS01:        ${total_cs01:,.2f}")
    print()
    print("Equity Risk:")
    print(f"  Total Delta:       {total_delta:,.2f}")
    print(f"  Total Gamma:       {total_gamma:.4f}")
    print(f"  Total Vega:        ${total_vega:,.2f}")
    print(f"  Total Theta:       ${total_theta:,.2f}")
    print("-"*80)
    print()
    
    # 8. Export to DataFrame
    print("8. Export to DataFrame")
    print("="*80)
    
    import polars as pl
    df = pl.DataFrame(risk_data)
    print(f"DataFrame shape: {df.shape}")
    print()
    print("Sample rows:")
    print(df.head(5))
    print()
    
    # Can export for further analysis
    # df.write_csv("risk_report.csv")
    # df.write_parquet("risk_report.parquet")
    
    print("="*80)
    print("EXAMPLE COMPLETE")
    print("="*80)
    print()
    print("Key Takeaways:")
    print("- DV01 measures interest rate sensitivity ($/bp)")
    print("- CS01 measures credit spread sensitivity ($/bp)")
    print("- Greeks measure option sensitivities (Delta, Gamma, Vega, Theta, Rho)")
    print("- Risk can be aggregated by maturity bucket, asset class, entity, etc.")
    print("- DataFrame export enables visualization and further analysis")


if __name__ == "__main__":
    main()

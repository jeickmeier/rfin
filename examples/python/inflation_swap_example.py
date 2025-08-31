#!/usr/bin/env python3
"""
Inflation Swap Valuation Example

This example demonstrates the valuation of a zero-coupon inflation swap using
the finstack library. It shows how to:

1. Create market data (discount curves, inflation index, inflation curve)
2. Build an inflation swap instrument 
3. Calculate present value and key metrics (breakeven inflation, leg PVs)
4. Analyze the swap from different perspectives (pay vs receive fixed)

The example uses industry-standard conventions and realistic market data
to illustrate the mathematical models implemented in the inflation swap.
"""

import finstack as fs
from datetime import date

def main():
    print("=== Inflation Swap Valuation Example ===\n")
    
    # 1. Set up market data
    print("1. Creating market data...")
    
    # Base date for valuation
    as_of = date(2025, 1, 1)
    
    # Create a USD OIS discount curve (nominal rates)
    # Points: (time_years, discount_factor)
    discount_curve = fs.DiscountCurve.builder("USD-OIS") \
        .base_date(as_of) \
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.88), (10.0, 0.75)]) \
        .monotone_convex() \
        .build()
    
    # Create historical inflation index (US CPI-U)
    # Historical observations: (date, index_value)
    inflation_observations = [
        (date(2024, 1, 1), 280.0),
        (date(2024, 7, 1), 285.0), 
        (date(2025, 1, 1), 290.0),
    ]
    
    inflation_index = fs.InflationIndex.new(
        "US-CPI-U", 
        inflation_observations, 
        fs.Currency.USD
    ).with_lag(fs.InflationLag.Months(3))  # Standard 3-month lag
    
    # Create forward inflation curve (market-implied inflation expectations)
    # Points: (time_years, projected_cpi_level)
    inflation_curve = fs.InflationCurve.builder("US-CPI-U") \
        .base_cpi(290.0) \
        .knots([(0.0, 290.0), (5.0, 320.0), (10.0, 355.0)]) \
        .log_df() \
        .build()
    
    # Combine into market context
    market = fs.MarketContext.new() \
        .with_discount(discount_curve) \
        .with_inflation_index("US-CPI-U", inflation_index) \
        .with_inflation(inflation_curve)
    
    print(f"✓ Market context created with valuation date: {as_of}")
    
    # 2. Create inflation swap instruments
    print("\n2. Creating inflation swap instruments...")
    
    start_date = date(2025, 1, 15)
    maturity_date = date(2030, 1, 15)
    notional = fs.Money.new(10_000_000.0, fs.Currency.USD)  # $10M notional
    
    # Swap A: Pay Fixed (receive inflation-linked payments)
    swap_pay_fixed = fs.InflationSwap.builder() \
        .id("ZCIS_PAY_FIXED") \
        .notional(notional) \
        .start(start_date) \
        .maturity(maturity_date) \
        .fixed_rate(0.025) \
        .inflation_id("US-CPI-U") \
        .disc_id("USD-OIS") \
        .dc(fs.DayCount.ActAct) \
        .side(fs.PayReceiveInflation.PayFixed) \
        .build()
    
    # Swap B: Receive Fixed (pay inflation-linked payments)  
    swap_receive_fixed = fs.InflationSwap.builder() \
        .id("ZCIS_RECEIVE_FIXED") \
        .notional(notional) \
        .start(start_date) \
        .maturity(maturity_date) \
        .fixed_rate(0.025) \
        .inflation_id("US-CPI-U") \
        .disc_id("USD-OIS") \
        .dc(fs.DayCount.ActAct) \
        .side(fs.PayReceiveInflation.ReceiveFixed) \
        .build()
    
    print(f"✓ Created swaps with {notional.amount()/1e6:.1f}M {notional.currency()} notional")
    print(f"  Term: {start_date} to {maturity_date} (~5 years)")
    print(f"  Fixed rate: {swap_pay_fixed.fixed_rate*100:.2f}% annual")
    
    # 3. Calculate present values
    print("\n3. Calculating present values...")
    
    # Present value calculations
    pv_pay_fixed = swap_pay_fixed.value(market, as_of)
    pv_receive_fixed = swap_receive_fixed.value(market, as_of)
    
    print(f"Pay Fixed Swap PV:     ${pv_pay_fixed.amount():>12,.0f}")
    print(f"Receive Fixed Swap PV: ${pv_receive_fixed.amount():>12,.0f}")
    print(f"Difference:            ${(pv_pay_fixed.amount() - pv_receive_fixed.amount()):>12,.0f}")
    
    # 4. Calculate key metrics
    print("\n4. Calculating key metrics...")
    
    # Create metric contexts for detailed analysis
    import finstack.metrics as metrics
    
    # Calculate breakeven inflation rate
    metric_context_pay = metrics.MetricContext.new(
        swap_pay_fixed, market, as_of, pv_pay_fixed
    )
    
    breakeven_calc = metrics.BreakevenCalculator()
    breakeven_rate = breakeven_calc.calculate(metric_context_pay)
    
    print(f"Breakeven Inflation Rate: {breakeven_rate*100:.3f}% annual")
    
    # Calculate individual leg PVs
    fixed_leg_calc = metrics.FixedLegPvCalculator()
    inflation_leg_calc = metrics.InflationLegPvCalculator()
    
    fixed_leg_pv = fixed_leg_calc.calculate(metric_context_pay)
    inflation_leg_pv = inflation_leg_calc.calculate(metric_context_pay)
    
    print(f"Fixed Leg PV:         ${fixed_leg_pv:>12,.0f}")
    print(f"Inflation Leg PV:     ${inflation_leg_pv:>12,.0f}")
    print(f"Net PV (Pay Fixed):   ${inflation_leg_pv - fixed_leg_pv:>12,.0f}")
    
    # Calculate sensitivities
    ir01_calc = metrics.Ir01Calculator()
    inflation01_calc = metrics.Inflation01Calculator()
    
    ir01 = ir01_calc.calculate(metric_context_pay)
    inflation01 = inflation01_calc.calculate(metric_context_pay)
    
    print(f"\nSensitivities (1bp parallel shifts):")
    print(f"IR01 (rate sensitivity):      ${ir01:>8,.0f}")
    print(f"Inflation01 (infl. sens.):    ${inflation01:>8,.0f}")
    
    # 5. Sensitivity analysis and market interpretation
    print("\n5. Market interpretation...")
    
    # Expected inflation calculation
    expected_inflation_total = (320.0 / 290.0) - 1.0  # From inflation curve
    expected_inflation_annual = ((320.0 / 290.0) ** (1/5)) - 1.0
    
    print(f"Market-implied inflation:")
    print(f"  Total over 5 years:   {expected_inflation_total*100:.1f}%")
    print(f"  Annualized rate:      {expected_inflation_annual*100:.2f}%")
    print(f"  Fixed rate:           {swap_pay_fixed.fixed_rate*100:.2f}%")
    
    if expected_inflation_annual > swap_pay_fixed.fixed_rate:
        print("  → Market expects inflation higher than fixed rate")
        print("  → Pay Fixed swap has positive value (receive > pay)")
    else:
        print("  → Market expects inflation lower than fixed rate") 
        print("  → Pay Fixed swap has negative value (receive < pay)")

if __name__ == "__main__":
    main()

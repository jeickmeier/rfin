#!/usr/bin/env python3
"""
Multi-curve yield curve calibration example.

Demonstrates post-2008 multi-curve framework with:
- OIS discount curve for discounting  
- Separate forward curves per tenor (1M, 3M, 6M)
- Market standard conventions
"""

import finstack as fs
from datetime import date, timedelta

def main():
    base_date = date(2025, 1, 1)
    
    print("Multi-Curve Calibration Example")
    print("=" * 50)
    
    # Step 1: Calibrate OIS discount curve
    print("\n1. Calibrating OIS discount curve...")
    
    # OIS quotes (deposits + OIS swaps)
    ois_quotes = [
        # Overnight deposit
        fs.RatesQuote.deposit(
            maturity=base_date + timedelta(days=1),
            rate=0.0450,
            day_count=fs.DayCount.Act365F
        ),
        # OIS swaps
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=30),
            rate=0.0452,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=90),
            rate=0.0455,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=180),
            rate=0.0458,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365),
            rate=0.0462,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365*2),
            rate=0.0468,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365*5),
            rate=0.0475,
            fixed_freq=fs.Frequency.annual(),
            float_freq=fs.Frequency.daily(),
            fixed_dc=fs.DayCount.Act365F,
            float_dc=fs.DayCount.Act365F,
            index="USD-OIS"
        ),
    ]
    
    # Create calibrator with Hagan-West monotone convex interpolation
    ois_calibrator = fs.DiscountCurveCalibrator(
        curve_id="USD-OIS-DISC",
        base_date=base_date,
        currency=fs.Currency.USD
    ).with_solve_interp(fs.InterpStyle.MonotoneConvex)
    
    # Calibrate OIS curve
    context = fs.MarketContext()
    ois_curve, ois_report = ois_calibrator.calibrate(ois_quotes, context)
    
    print(f"  Success: {ois_report.success}")
    print(f"  Iterations: {ois_report.iterations}")
    print(f"  Max residual: {ois_report.max_residual:.2e}")
    print(f"  RMSE: {ois_report.rmse:.2e}")
    
    # Update context with OIS curve and collateral mapping
    context = context.insert_discount(ois_curve).map_collateral("USD-CSA", "USD-OIS-DISC")
    
    # Step 2: Calibrate 3M forward curve
    print("\n2. Calibrating 3M SOFR forward curve...")
    
    # 3M forward quotes (FRAs + futures + vanilla IRS)
    forward_3m_quotes = [
        # FRAs
        fs.RatesQuote.fra(
            start=base_date + timedelta(days=30),
            end=base_date + timedelta(days=120),
            rate=0.0463,
            day_count=fs.DayCount.Act360
        ),
        fs.RatesQuote.fra(
            start=base_date + timedelta(days=90),
            end=base_date + timedelta(days=180),
            rate=0.0465,
            day_count=fs.DayCount.Act360
        ),
        fs.RatesQuote.fra(
            start=base_date + timedelta(days=180),
            end=base_date + timedelta(days=270),
            rate=0.0468,
            day_count=fs.DayCount.Act360
        ),
        # Futures
        fs.RatesQuote.future(
            expiry=base_date + timedelta(days=90),
            price=95.30,  # Implies 4.70% rate
            specs=fs.FutureSpecs(
                multiplier=2500.0,
                face_value=1_000_000.0,
                delivery_months=3,
                day_count=fs.DayCount.Act360,
                convexity_adjustment=0.0002  # 2bp convexity adjustment
            )
        ),
        # Vanilla IRS vs 3M SOFR
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365),
            rate=0.0470,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="USD-SOFR-3M"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365*2),
            rate=0.0475,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="USD-SOFR-3M"
        ),
        fs.RatesQuote.swap(
            maturity=base_date + timedelta(days=365*5),
            rate=0.0482,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="USD-SOFR-3M"
        ),
    ]
    
    # Create forward curve calibrator
    forward_3m_calibrator = fs.ForwardCurveCalibrator(
        fwd_curve_id="USD-SOFR-3M-FWD",
        tenor_years=0.25,  # 3 months = 0.25 years
        base_date=base_date,
        currency=fs.Currency.USD,
        discount_curve_id="USD-OIS-DISC"
    ).with_solve_interp(fs.InterpStyle.Linear)
    
    # Calibrate forward curve
    forward_3m_curve, forward_3m_report = forward_3m_calibrator.calibrate(
        forward_3m_quotes, context
    )
    
    print(f"  Success: {forward_3m_report.success}")
    print(f"  Iterations: {forward_3m_report.iterations}")
    print(f"  Max residual: {forward_3m_report.max_residual:.2e}")
    print(f"  RMSE: {forward_3m_report.rmse:.2e}")
    
    context = context.insert_forward(forward_3m_curve)
    
    # Step 3: Display curve values
    print("\n3. Curve Values")
    print("-" * 30)
    
    disc_curve = context.disc("USD-OIS-DISC")
    fwd_3m_curve = context.fwd("USD-SOFR-3M-FWD")
    
    print("\nOIS Discount Factors:")
    for t in [0.25, 0.5, 1.0, 2.0, 5.0]:
        print(f"  t = {t:4.2f} years: DF = {disc_curve.df(t):.6f}")
    
    print("\n3M Forward Rates:")
    for t in [0.0, 0.25, 0.5, 1.0, 2.0, 5.0]:
        print(f"  t = {t:4.2f} years: Fwd = {fwd_3m_curve.rate(t)*100:.2f}%")
    
    # Step 4: Demonstrate basis spreads (conceptual)
    print("\n4. Basis Spreads")
    print("-" * 30)
    print("In a full implementation, we would:")
    print("  - Calibrate 1M and 6M forward curves")
    print("  - Use basis swaps to ensure tenor consistency")
    print("  - Apply basis adjustments to align curves")
    
    # Example basis swap quote (for illustration)
    basis_swap = fs.RatesQuote.basis_swap(
        maturity=base_date + timedelta(days=365*2),
        primary_index="USD-SOFR-3M",
        reference_index="USD-SOFR-6M",
        spread_bp=2.5,  # 3M pays 6M + 2.5bp
        primary_freq=fs.Frequency.quarterly(),
        reference_freq=fs.Frequency.semi_annual(),
        primary_dc=fs.DayCount.Act360,
        reference_dc=fs.DayCount.Act360,
        currency=fs.Currency.USD
    )
    print(f"\nExample basis swap: 3M vs 6M spread = {basis_swap.spread_bp}bp")
    
    # Step 5: Simple calibration alternative
    print("\n5. Alternative: SimpleCalibration")
    print("-" * 30)
    print("For convenience, use SimpleCalibration to calibrate all curves at once:")
    
    # Combine all quotes
    all_quotes = [fs.MarketQuote.rates(q) for q in ois_quotes + forward_3m_quotes]
    
    # Use SimpleCalibration for one-shot calibration
    simple_cal = fs.SimpleCalibration(base_date, fs.Currency.USD)
    final_context, final_report = simple_cal.calibrate(all_quotes)
    
    print(f"  Total iterations: {final_report.iterations}")
    print(f"  Success: {final_report.success}")
    
    print("\n" + "=" * 50)
    print("Multi-curve calibration complete!")
    print("\nMarket context contains:")
    print("  - Discount curve: USD-OIS-DISC")
    print("  - Forward curve: USD-SOFR-3M-FWD")
    print("  - Collateral mapping: USD-CSA -> USD-OIS-DISC")

if __name__ == "__main__":
    main()

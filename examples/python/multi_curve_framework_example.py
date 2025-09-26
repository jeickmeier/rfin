#!/usr/bin/env python3
"""
Multi-Curve Framework Calibration Example

Demonstrates proper calibration workflow for multi-curve framework where
OIS discount curves and forward curves are calibrated separately.
"""

import finstack as fs
from datetime import date, timedelta

def create_ois_quotes(base_date: date) -> list:
    """Create OIS quotes for discount curve calibration."""
    quotes = []
    
    # Overnight deposits
    quotes.append(fs.RatesQuote.deposit(
        maturity=base_date + timedelta(days=1),
        rate=0.0150,
        day_count=fs.DayCount.Act360
    ))
    
    # OIS swaps (SOFR-based)
    tenors = [
        (3, 0.0155),   # 3M
        (6, 0.0160),   # 6M
        (12, 0.0170),  # 1Y
        (24, 0.0185),  # 2Y
        (36, 0.0195),  # 3Y
        (60, 0.0210),  # 5Y
        (84, 0.0220),  # 7Y
        (120, 0.0230), # 10Y
    ]
    
    for months, rate in tenors:
        maturity = base_date + timedelta(days=30 * months)
        quotes.append(fs.RatesQuote.swap(
            maturity=maturity,
            rate=rate,
            fixed_freq=fs.Frequency.Annual,
            float_freq=fs.Frequency.Daily,  # OIS typically compounds daily
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="SOFR"  # OIS index
        ))
    
    return quotes

def create_libor_quotes(base_date: date) -> list:
    """Create LIBOR quotes for forward curve calibration."""
    quotes = []
    
    # FRAs for short end
    fra_data = [
        (1, 4, 0.0170),   # 1x4 FRA
        (2, 5, 0.0175),   # 2x5 FRA
        (3, 6, 0.0180),   # 3x6 FRA
        (6, 9, 0.0190),   # 6x9 FRA
        (9, 12, 0.0195),  # 9x12 FRA
    ]
    
    for start_months, end_months, rate in fra_data:
        start = base_date + timedelta(days=30 * start_months)
        end = base_date + timedelta(days=30 * end_months)
        quotes.append(fs.RatesQuote.fra(
            start=start,
            end=end,
            rate=rate,
            day_count=fs.DayCount.Act360
        ))
    
    # LIBOR swaps for longer tenors
    swap_tenors = [
        (24, 0.0205),  # 2Y
        (36, 0.0215),  # 3Y
        (60, 0.0235),  # 5Y
        (84, 0.0245),  # 7Y
        (120, 0.0255), # 10Y
    ]
    
    for months, rate in swap_tenors:
        maturity = base_date + timedelta(days=30 * months)
        quotes.append(fs.RatesQuote.swap(
            maturity=maturity,
            rate=rate,
            fixed_freq=fs.Frequency.SemiAnnual,
            float_freq=fs.Frequency.Quarterly,  # 3M LIBOR
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-LIBOR"
        ))
    
    return quotes

def demonstrate_single_curve_mode():
    """Single-curve mode has been removed; use multi-curve calibration only."""
    print("\n" + "="*60)
    print("SINGLE-CURVE MODE REMOVED")
    print("="*60)
    print("This example has been deprecated. Please use the multi-curve workflow.")

def demonstrate_multi_curve_mode():
    """Demonstrate multi-curve calibration (post-2008 methodology)."""
    print("\n" + "="*60)
    print("MULTI-CURVE MODE CALIBRATION")
    print("="*60)
    
    base_date = date(2024, 1, 1)
    
    # Create configuration for multi-curve mode
    config = fs.CalibrationConfig(
        tolerance=1e-8,
        max_iterations=100,
        multi_curve=fs.MultiCurveConfig.multi_curve()
    )
    
    # Initialize context
    context = fs.MarketContext(base_date)
    
    # Step 1: Calibrate OIS discount curve
    print("\nStep 1: Calibrating OIS Discount Curve")
    print("-" * 40)
    
    ois_quotes = create_ois_quotes(base_date)
    print(f"Using {len(ois_quotes)} OIS instruments")
    
    disc_calibrator = fs.DiscountCurveCalibrator(
        base_date=base_date,
        currency=fs.Currency.USD,
        config=config
    )
    
    try:
        discount_curve, disc_report = disc_calibrator.calibrate(ois_quotes, context)
        discount_curve.id = "OIS"  # Set curve ID
        context = context.insert_discount(discount_curve)
        
        print(f"✓ OIS discount curve calibrated")
        print(f"  Iterations: {disc_report.iterations}")
        
        # Show some discount factors
        for t in [0.25, 1.0, 5.0, 10.0]:
            df = discount_curve.df(t)
            rate = discount_curve.zero_rate(t)
            print(f"  T={t:5.2f}y: DF={df:.6f}, OIS Rate={rate*100:.3f}%")
            
    except Exception as e:
        print(f"✗ OIS calibration failed: {e}")
        return
    
    # Step 2: Calibrate 3M LIBOR forward curve
    print("\nStep 2: Calibrating 3M LIBOR Forward Curve")
    print("-" * 40)
    
    libor_quotes = create_libor_quotes(base_date)
    print(f"Using {len(libor_quotes)} LIBOR instruments")
    
    fwd_calibrator = fs.ForwardCurveCalibrator(
        fwd_curve_id="3M-LIBOR",
        tenor_years=0.25,  # 3-month tenor
        base_date=base_date,
        currency=fs.Currency.USD,
        discount_curve_id="OIS",
        config=config
    )
    
    try:
        forward_curve, fwd_report = fwd_calibrator.calibrate(libor_quotes, context)
        context = context.insert_forward(forward_curve)
        
        print(f"✓ 3M LIBOR forward curve calibrated")
        print(f"  Iterations: {fwd_report.iterations}")
        
        # Show some forward rates
        for start in [0.0, 0.25, 1.0, 2.0, 5.0]:
            end = start + 0.25  # 3M forward
            fwd_rate = forward_curve.fwd(start, end)
            print(f"  F({start:.2f},{end:.2f}): {fwd_rate*100:.3f}%")
            
        # Calculate LIBOR-OIS spread
        print("\nLIBOR-OIS Spreads:")
        for t in [0.25, 1.0, 2.0, 5.0]:
            ois_rate = discount_curve.zero_rate(t)
            # Approximate LIBOR rate from forward curve
            libor_rate = forward_curve.fwd(0, t)
            spread_bp = (libor_rate - ois_rate) * 10000
            print(f"  T={t:4.2f}y: Spread={spread_bp:.1f}bp")
            
    except Exception as e:
        print(f"✗ Forward curve calibration failed: {e}")

def demonstrate_incorrect_multi_curve():
    """Demonstrate what happens with incorrect multi-curve setup."""
    print("\n" + "="*60)
    print("INCORRECT MULTI-CURVE SETUP (FOR COMPARISON)")
    print("="*60)
    
    base_date = date(2024, 1, 1)
    
    # Create configuration for multi-curve mode
    config = fs.CalibrationConfig(
        tolerance=1e-8,
        max_iterations=100,
        multi_curve=fs.MultiCurveConfig.multi_curve()
    )
    
    # Try to calibrate discount curve with instruments that need forward curves
    print("\nAttempting to calibrate with forward-dependent instruments...")
    print("-" * 40)
    
    # Create quotes that include FRAs and LIBOR swaps (incorrect for OIS calibration)
    bad_quotes = []
    
    # Add a deposit (this is OK)
    bad_quotes.append(fs.RatesQuote.deposit(
        maturity=base_date + timedelta(days=7),
        rate=0.0150,
        day_count=fs.DayCount.Act360
    ))
    
    # Add FRAs (these need forward curves!)
    for start_months, end_months, rate in [(3, 6, 0.018), (6, 9, 0.019)]:
        start = base_date + timedelta(days=30 * start_months)
        end = base_date + timedelta(days=30 * end_months)
        bad_quotes.append(fs.RatesQuote.fra(
            start=start,
            end=end,
            rate=rate,
            day_count=fs.DayCount.Act360
        ))
    
    # Add LIBOR swaps (these also need forward curves!)
    for months, rate in [(12, 0.020), (24, 0.022)]:
        maturity = base_date + timedelta(days=30 * months)
        bad_quotes.append(fs.RatesQuote.swap(
            maturity=maturity,
            rate=rate,
            fixed_freq=fs.Frequency.SemiAnnual,
            float_freq=fs.Frequency.Quarterly,
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-LIBOR"  # Not an OIS index!
        ))
    
    calibrator = fs.DiscountCurveCalibrator(
        base_date=base_date,
        currency=fs.Currency.USD,
        config=config
    )
    
    context = fs.MarketContext(base_date)
    
    try:
        # This should fail or produce warnings
        discount_curve, report = calibrator.calibrate(bad_quotes, context)
        print("⚠ Calibration completed but may be incorrect!")
        print("  The calibration used forward-dependent instruments without")
        print("  a forward curve in the context. Results may be unreliable.")
    except Exception as e:
        print(f"✓ Calibration correctly failed: {e}")
        print("  This is expected when using forward-dependent instruments")
        print("  without providing the necessary forward curves.")

def main():
    """Run all demonstrations."""
    print("\n" + "="*60)
    print("MULTI-CURVE FRAMEWORK CALIBRATION EXAMPLES")
    print("="*60)
    
    # Single-curve mode removed
    demonstrate_single_curve_mode()
    
    # Show correct multi-curve mode
    demonstrate_multi_curve_mode()
    
    # Show incorrect setup for comparison
    demonstrate_incorrect_multi_curve()
    
    print("\n" + "="*60)
    print("KEY TAKEAWAYS")
    print("="*60)
    print("""
1. Multi-Curve Mode:
   - Discount and forward curves calibrated separately
   - Captures basis spreads and funding costs
   - Required for accurate modern pricing

2. Instrument Selection:
   - OIS curves: Use deposits and OIS swaps
   - Forward curves: Use FRAs, futures, and tenor-specific swaps
   - Never mix OIS and LIBOR instruments in the same calibration

3. Calibration Order:
   - Always calibrate discount curve first
   - Then calibrate forward curves with discount in context
   - Validate instrument appropriateness for each curve type
""")

if __name__ == "__main__":
    main()

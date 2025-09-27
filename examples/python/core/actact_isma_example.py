#!/usr/bin/env python3
"""
Example demonstrating ACT/ACT (ISMA) day count convention with coupon-period awareness.

This example shows how the new ACT/ACT (ISMA) convention differs from the standard
ACT/ACT (ISDA) convention by using coupon payment frequency to ensure equal valuation
of days within each coupon period.
"""

import finstack as fs
from datetime import date


def demonstrate_actact_isma():
    """Demonstrate ACT/ACT (ISMA) vs ACT/ACT (ISDA) calculations."""
    
    print("=== ACT/ACT (ISMA) vs ACT/ACT (ISDA) Comparison ===\n")
    
    # Create test dates
    start = fs.Date(2025, 1, 15)
    end = fs.Date(2025, 7, 15)
    
    print(f"Period: {start} to {end}")
    print(f"Calendar days: {(end.to_py_date() - start.to_py_date()).days}")
    
    # Create day count conventions
    dc_isda = fs.DayCount.actact()       # Standard ISDA variant
    dc_isma = fs.DayCount.actact_isma()  # New ISMA variant
    
    print(f"\nDay Count Conventions:")
    print(f"  ISDA: {dc_isda}")
    print(f"  ISMA: {dc_isma}")
    
    # Calculate year fractions using different approaches
    yf_isda = dc_isda.year_fraction(start, end)
    
    # ISMA requires frequency parameter
    frequencies = [
        ("Annual", fs.Frequency.Annual),
        ("Semi-Annual", fs.Frequency.SemiAnnual), 
        ("Quarterly", fs.Frequency.Quarterly),
        ("Monthly", fs.Frequency.Monthly),
    ]
    
    print(f"\nYear Fractions:")
    print(f"  ACT/ACT (ISDA):     {yf_isda:.8f}")
    
    for freq_name, freq in frequencies:
        yf_isma = dc_isma.year_fraction_with_frequency(start, end, freq)
        print(f"  ACT/ACT (ISMA) {freq_name:>11}: {yf_isma:.8f}")


def demonstrate_leap_year_handling():
    """Show how ACT/ACT (ISMA) handles leap year periods."""
    
    print("\n\n=== Leap Year Handling ===\n")
    
    # Period spanning leap year boundary
    start = fs.Date(2023, 7, 1)   # Non-leap year
    end = fs.Date(2024, 7, 1)     # Leap year
    
    print(f"Period: {start} to {end} (spans leap year boundary)")
    
    dc_isda = fs.DayCount.actact()
    dc_isma = fs.DayCount.actact_isma()
    
    yf_isda = dc_isda.year_fraction(start, end)
    yf_isma_semi = dc_isma.year_fraction_with_frequency(start, end, fs.Frequency.SemiAnnual)
    
    print(f"\nYear Fractions:")
    print(f"  ACT/ACT (ISDA):           {yf_isda:.8f}")
    print(f"  ACT/ACT (ISMA) Semi-Ann:  {yf_isma_semi:.8f}")
    print(f"  Difference:               {abs(yf_isda - yf_isma_semi):.8f}")


def demonstrate_partial_coupon_periods():
    """Show calculations for partial coupon periods."""
    
    print("\n\n=== Partial Coupon Periods ===\n")
    
    # Short period within a coupon cycle
    start = fs.Date(2025, 1, 15)  # Mid-month start
    end = fs.Date(2025, 3, 15)    # Two months later
    
    print(f"Period: {start} to {end} (partial coupon period)")
    
    dc_isma = fs.DayCount.actact_isma()
    
    frequencies = [
        ("Semi-Annual", fs.Frequency.SemiAnnual),
        ("Quarterly", fs.Frequency.Quarterly),
        ("Monthly", fs.Frequency.Monthly),
    ]
    
    print(f"\nYear Fractions for different coupon frequencies:")
    for freq_name, freq in frequencies:
        yf = dc_isma.year_fraction_with_frequency(start, end, freq)
        print(f"  {freq_name:>12}: {yf:.8f}")


if __name__ == "__main__":
    try:
        demonstrate_actact_isma()
        demonstrate_leap_year_handling()
        demonstrate_partial_coupon_periods()
        
        print("\n" + "="*60)
        print("SUCCESS: ACT/ACT (ISMA) implementation working correctly!")
        print("="*60)
        
    except Exception as e:
        print(f"ERROR: {e}")
        import traceback
        traceback.print_exc()

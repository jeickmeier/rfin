#!/usr/bin/env python3
"""
Example demonstrating curve serialization with interpolators.

This example shows how to:
1. Create discount, forward, and inflation curves with various interpolation styles
2. Serialize them to JSON for persistence
3. Deserialize them back and verify accuracy
"""

import json
from datetime import date
from finstack import (
    DiscountCurve,
    ForwardCurve,
    InflationCurve,
    InterpStyle,
    ExtrapolationPolicy,
    DayCount,
)


def main():
    print("Curve Serialization Example")
    print("=" * 50)
    
    # 1. Create a discount curve with MonotoneConvex interpolation
    print("\n1. Creating discount curve with MonotoneConvex interpolation...")
    discount_curve = DiscountCurve.builder("USD-OIS") \
        .base_date(date(2025, 1, 1)) \
        .knots([
            (0.0, 1.0),
            (0.5, 0.99),
            (1.0, 0.975),
            (2.0, 0.95),
            (5.0, 0.88),
            (10.0, 0.75)
        ]) \
        .set_interp(InterpStyle.MonotoneConvex) \
        .extrapolation(ExtrapolationPolicy.FlatForward) \
        .build()
    
    # Test interpolation
    test_times = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0]
    original_dfs = {t: discount_curve.df(t) for t in test_times}
    
    print(f"Original DFs at key points:")
    for t, df in original_dfs.items():
        print(f"  t={t:5.2f}: {df:.6f}")
    
    # 2. Serialize the discount curve to JSON
    print("\n2. Serializing discount curve to JSON...")
    discount_json = discount_curve.to_json()
    print(f"JSON size: {len(discount_json)} bytes")
    
    # Pretty print a portion of the JSON
    discount_dict = json.loads(discount_json)
    print(f"Curve ID: {discount_dict.get('id', 'N/A')}")
    print(f"Interpolation type: {discount_dict.get('interp_data', {}).get('type', 'N/A')}")
    print(f"Number of knots: {len(discount_dict.get('knots', []))}")
    
    # 3. Deserialize back and verify accuracy
    print("\n3. Deserializing and verifying accuracy...")
    restored_curve = DiscountCurve.from_json(discount_json)
    
    max_error = 0.0
    for t in test_times:
        original_df = original_dfs[t]
        restored_df = restored_curve.df(t)
        error = abs(original_df - restored_df)
        max_error = max(max_error, error)
        if error > 1e-12:
            print(f"  Warning: Error at t={t}: {error}")
    
    if max_error < 1e-12:
        print(f"  ✓ Perfect reconstruction! Max error: {max_error:.2e}")
    else:
        print(f"  ✗ Reconstruction error: {max_error:.2e}")
    
    # 4. Create and serialize a forward curve
    print("\n4. Creating and serializing forward curve...")
    forward_curve = ForwardCurve.builder("USD-SOFR3M", 0.25) \
        .base_date(date(2025, 1, 1)) \
        .reset_lag(2) \
        .day_count(DayCount.Act360) \
        .knots([
            (0.0, 0.03),
            (0.25, 0.032),
            (0.5, 0.035),
            (1.0, 0.04),
            (2.0, 0.042),
            (5.0, 0.045)
        ]) \
        .set_interp(InterpStyle.Linear) \
        .build()
    
    forward_json = forward_curve.to_json()
    restored_forward = ForwardCurve.from_json(forward_json)
    
    # Test forward rates
    print("Forward rates comparison:")
    for t in [0.0, 0.5, 1.0, 2.0, 5.0]:
        original_rate = forward_curve.rate(t)
        restored_rate = restored_forward.rate(t)
        print(f"  t={t}: Original={original_rate:.4%}, Restored={restored_rate:.4%}")
    
    # 5. Create and serialize an inflation curve
    print("\n5. Creating and serializing inflation curve...")
    inflation_curve = InflationCurve.builder("US-CPI") \
        .base_cpi(300.0) \
        .knots([
            (0.0, 300.0),
            (1.0, 306.0),
            (2.0, 312.5),
            (5.0, 330.0),
            (10.0, 360.0)
        ]) \
        .set_interp(InterpStyle.LogLinear) \
        .build()
    
    inflation_json = inflation_curve.to_json()
    restored_inflation = InflationCurve.from_json(inflation_json)
    
    # Test CPI levels and inflation rates
    print("CPI levels comparison:")
    for t in [0.0, 1.0, 2.0, 5.0, 10.0]:
        original_cpi = inflation_curve.cpi(t)
        restored_cpi = restored_inflation.cpi(t)
        print(f"  t={t}: Original={original_cpi:.2f}, Restored={restored_cpi:.2f}")
    
    print("\nInflation rates comparison:")
    for t1, t2 in [(0, 1), (1, 2), (2, 5), (5, 10)]:
        original_rate = inflation_curve.inflation_rate(t1, t2)
        restored_rate = restored_inflation.inflation_rate(t1, t2)
        print(f"  [{t1},{t2}]Y: Original={original_rate:.2%}, Restored={restored_rate:.2%}")
    
    # 6. Test different interpolation styles
    print("\n6. Testing all interpolation styles...")
    styles = [
        InterpStyle.Linear,
        InterpStyle.LogLinear,
        InterpStyle.CubicHermite,
        InterpStyle.FlatFwd,
        InterpStyle.MonotoneConvex,  # Only for discount curves with decreasing values
    ]
    
    for style in styles:
        try:
            # Use appropriate curve type for MonotoneConvex
            if style == InterpStyle.MonotoneConvex:
                curve = DiscountCurve.builder(f"TEST-{style.name}") \
                    .base_date(date(2025, 1, 1)) \
                    .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)]) \
                    .set_interp(style) \
                    .build()
            else:
                curve = DiscountCurve.builder(f"TEST-{style.name}") \
                    .base_date(date(2025, 1, 1)) \
                    .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)]) \
                    .set_interp(style) \
                    .build()
            
            # Serialize and restore
            json_str = curve.to_json()
            restored = DiscountCurve.from_json(json_str)
            
            # Check a test point
            test_t = 0.5
            original_val = curve.df(test_t)
            restored_val = restored.df(test_t)
            error = abs(original_val - restored_val)
            
            status = "✓" if error < 1e-12 else "✗"
            print(f"  {status} {style.name:15} - Error: {error:.2e}")
            
        except Exception as e:
            print(f"  ✗ {style.name:15} - Failed: {e}")
    
    # 7. Save curves to files
    print("\n7. Saving curves to files...")
    
    # Save discount curve
    with open("discount_curve.json", "w") as f:
        json.dump(json.loads(discount_json), f, indent=2)
    print("  Saved discount_curve.json")
    
    # Save forward curve
    with open("forward_curve.json", "w") as f:
        json.dump(json.loads(forward_json), f, indent=2)
    print("  Saved forward_curve.json")
    
    # Save inflation curve
    with open("inflation_curve.json", "w") as f:
        json.dump(json.loads(inflation_json), f, indent=2)
    print("  Saved inflation_curve.json")
    
    print("\n✓ Curve serialization example completed successfully!")


if __name__ == "__main__":
    main()

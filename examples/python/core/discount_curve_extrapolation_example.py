#!/usr/bin/env python3
"""
Example demonstrating DiscountCurve extrapolation policies and monotonic validation.

This example shows:
1. How to configure extrapolation policies (flat-zero vs flat-forward)
2. When and how to use monotonic validation for credit curves
3. Practical differences between extrapolation methods
4. Credit curve construction with proper validation
"""

import numpy as np
import matplotlib.pyplot as plt
from finstack import Date
from finstack.market_data import DiscountCurve, InterpStyle, ExtrapolationPolicy

def main():
    # Base parameters
    base_date = Date(2025, 1, 1)
    times = [0.0, 0.5, 1.0, 2.0, 5.0]
    discount_factors = [1.0, 0.975, 0.95, 0.90, 0.78]
    
    print("=== DiscountCurve Extrapolation Policy Demo ===")
    print()
    
    # 1. Compare extrapolation policies
    print("1. Extrapolation Policy Comparison")
    print("-" * 40)
    
    # Create curves with different extrapolation policies
    curve_flat_zero = DiscountCurve(
        id="USD-OIS-FLAT-ZERO",
        base_date=base_date,
        times=times,
        discount_factors=discount_factors,
        interpolation=InterpStyle.MonotoneConvex,
        extrapolation=ExtrapolationPolicy.FlatZero
    )
    
    curve_flat_forward = DiscountCurve(
        id="USD-OIS-FLAT-FWD", 
        base_date=base_date,
        times=times,
        discount_factors=discount_factors,
        interpolation=InterpStyle.MonotoneConvex,
        extrapolation=ExtrapolationPolicy.FlatForward
    )
    
    # Test extrapolation behavior
    extrap_times = [-1.0, -0.5, 7.0, 10.0, 20.0]
    print(f"{'Time':<8} {'Flat-Zero':<12} {'Flat-Forward':<12} {'Difference':<12}")
    print("-" * 50)
    
    for t in extrap_times:
        df_flat_zero = curve_flat_zero.df(t)
        df_flat_forward = curve_flat_forward.df(t)
        diff = df_flat_forward - df_flat_zero
        print(f"{t:<8.1f} {df_flat_zero:<12.6f} {df_flat_forward:<12.6f} {diff:<12.6f}")
    
    print()
    
    # 2. Monotonic validation for credit curves
    print("2. Credit Curve Monotonic Validation")
    print("-" * 40)
    
    # Valid credit curve (strictly decreasing)
    credit_times = [0.0, 1.0, 3.0, 5.0, 10.0]
    survival_probs = [1.0, 0.992, 0.970, 0.940, 0.860]  # Decreasing
    
    try:
        credit_curve = DiscountCurve(
            id="CREDIT-CORP-AA",
            base_date=base_date,
            times=credit_times,
            discount_factors=survival_probs,
            interpolation=InterpStyle.MonotoneConvex,
            extrapolation=ExtrapolationPolicy.FlatForward,
            require_monotonic=True
        )
        print("✓ Valid credit curve created successfully")
        
        # Show survival probabilities at various times
        test_times = [0.5, 2.0, 7.0, 15.0]
        print(f"{'Time':<8} {'Survival Prob':<15} {'Note':<20}")
        print("-" * 45)
        for t in test_times:
            sp = credit_curve.df(t)
            note = "extrapolated" if t > max(credit_times) else "interpolated"
            print(f"{t:<8.1f} {sp:<15.6f} {note:<20}")
            
    except ValueError as e:
        print(f"✗ Credit curve validation failed: {e}")
    
    print()
    
    # Try invalid credit curve (non-monotonic)
    print("3. Invalid Credit Curve (Non-Monotonic)")
    print("-" * 40)
    
    invalid_survival_probs = [1.0, 0.992, 0.970, 0.975, 0.860]  # Increases at t=5
    
    try:
        invalid_curve = DiscountCurve(
            id="INVALID-CREDIT",
            base_date=base_date,
            times=credit_times,
            discount_factors=invalid_survival_probs,
            interpolation=InterpStyle.MonotoneConvex,
            require_monotonic=True
        )
        print("✗ This should not succeed!")
    except ValueError as e:
        print(f"✓ Correctly rejected invalid curve: {e}")
    
    print()
    
    # 4. Visualization of extrapolation differences
    print("4. Generating Visualization...")
    print("-" * 40)
    
    # Extended time range for visualization
    viz_times = np.linspace(-2, 15, 100)
    
    flat_zero_dfs = [curve_flat_zero.df(t) for t in viz_times]
    flat_forward_dfs = [curve_flat_forward.df(t) for t in viz_times]
    
    plt.figure(figsize=(12, 8))
    
    # Plot curves
    plt.subplot(2, 1, 1)
    plt.plot(viz_times, flat_zero_dfs, 'b-', label='Flat-Zero Extrapolation', linewidth=2)
    plt.plot(viz_times, flat_forward_dfs, 'r--', label='Flat-Forward Extrapolation', linewidth=2)
    plt.scatter(times, discount_factors, color='black', s=50, zorder=5, label='Market Data')
    
    plt.axvline(x=times[0], color='gray', linestyle=':', alpha=0.7, label='Data Range')
    plt.axvline(x=times[-1], color='gray', linestyle=':', alpha=0.7)
    
    plt.xlabel('Time (years)')
    plt.ylabel('Discount Factor')
    plt.title('DiscountCurve Extrapolation Policy Comparison')
    plt.legend()
    plt.grid(True, alpha=0.3)
    
    # Plot difference
    plt.subplot(2, 1, 2)
    diff = np.array(flat_forward_dfs) - np.array(flat_zero_dfs)
    plt.plot(viz_times, diff, 'g-', linewidth=2)
    plt.axhline(y=0, color='black', linestyle='-', alpha=0.5)
    plt.axvline(x=times[0], color='gray', linestyle=':', alpha=0.7)
    plt.axvline(x=times[-1], color='gray', linestyle=':', alpha=0.7)
    
    plt.xlabel('Time (years)')
    plt.ylabel('Difference (Flat-Forward - Flat-Zero)')
    plt.title('Extrapolation Policy Difference')
    plt.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig('discount_curve_extrapolation_comparison.png', dpi=150, bbox_inches='tight')
    print("✓ Saved visualization as 'discount_curve_extrapolation_comparison.png'")
    
    print()
    
    # 5. Zero rate comparison
    print("5. Zero Rate Extrapolation Analysis")
    print("-" * 40)
    
    test_times_zero = [0.5, 1.0, 2.0, 7.0, 10.0, 20.0]
    print(f"{'Time':<8} {'Flat-Zero':<12} {'Flat-Forward':<12} {'Spread (bps)':<12}")
    print("-" * 50)
    
    for t in test_times_zero:
        if t > 0:  # Avoid division by zero
            zero_flat_zero = -np.log(curve_flat_zero.df(t)) / t
            zero_flat_forward = -np.log(curve_flat_forward.df(t)) / t
            spread_bps = (zero_flat_forward - zero_flat_zero) * 10000
            print(f"{t:<8.1f} {zero_flat_zero:<12.4f} {zero_flat_forward:<12.4f} {spread_bps:<12.1f}")
    
    print()
    print("Key Insights:")
    print("• Flat-zero extrapolation is conservative (rates become zero)")
    print("• Flat-forward extrapolation maintains market-consistent forward rates")
    print("• Credit curves should use flat-forward with monotonic validation")
    print("• Choice depends on use case: pricing vs risk management")

if __name__ == "__main__":
    main()

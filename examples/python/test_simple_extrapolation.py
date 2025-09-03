#!/usr/bin/env python3
"""Simple test to verify extrapolation is working correctly."""

from finstack import Date
from finstack.market_data import DiscountCurve, InterpStyle, ExtrapolationPolicy

# Create two curves with different extrapolation policies
base_date = Date(2025, 1, 1)
times = [0.0, 1.0, 2.0]
discount_factors = [1.0, 0.95, 0.90]

curve_flat_zero = DiscountCurve(
    id="FLAT-ZERO",
    base_date=base_date,
    times=times,
    discount_factors=discount_factors,
    interpolation=InterpStyle.LogLinear,
    extrapolation=ExtrapolationPolicy.FlatZero
)

curve_flat_forward = DiscountCurve(
    id="FLAT-FORWARD", 
    base_date=base_date,
    times=times,
    discount_factors=discount_factors,
    interpolation=InterpStyle.LogLinear,
    extrapolation=ExtrapolationPolicy.FlatForward
)

# Test extrapolation beyond the curve
t_test = 5.0  # Beyond last knot at t=2.0

df_flat_zero = curve_flat_zero.df(t_test)
df_flat_forward = curve_flat_forward.df(t_test)

print(f"At t={t_test}:")
print(f"Flat-Zero DF: {df_flat_zero:.6f}")
print(f"Flat-Forward DF: {df_flat_forward:.6f}")
print(f"Difference: {df_flat_forward - df_flat_zero:.6f}")

# Test left extrapolation too
t_test_left = -1.0

df_left_flat_zero = curve_flat_zero.df(t_test_left)
df_left_flat_forward = curve_flat_forward.df(t_test_left)

print(f"\nAt t={t_test_left}:")
print(f"Flat-Zero DF: {df_left_flat_zero:.6f}")
print(f"Flat-Forward DF: {df_left_flat_forward:.6f}")
print(f"Difference: {df_left_flat_forward - df_left_flat_zero:.6f}")

print(f"\nExtrapolation policies:")
print(f"Curve 1: {curve_flat_zero.id} - should use FlatZero")
print(f"Curve 2: {curve_flat_forward.id} - should use FlatForward")

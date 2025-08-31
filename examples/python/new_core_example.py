#!/usr/bin/env python3
"""
Example demonstrating core finstack functionality including:
- Period generation for financial reporting
- InflationIndex for CPI/RPI calculations (NEW!)
"""

from finstack import Date, Currency
from finstack.dates import PeriodId, build_periods

# ============================================================
# Period Generation for Financial Reporting
# ============================================================

print("=== Period Generation ===")
periods = build_periods("2025Q1..2028Q4", "2026Q2")
for p in periods[:4]:  # Show first 4 periods
    print(f"  {p.id}: actual={p.is_actual}")
print(f"  ... ({len(periods)} total periods)")

# ============================================================
# NEW: InflationIndex API (replaces deprecated IndexSeries)
# ============================================================

print("\n=== InflationIndex API (NEW!) ===")
print("IndexSeries has been replaced with InflationIndex using Polars DataFrames")

from finstack.market_data import InflationIndex, InflationInterpolation, InflationLag

# Historical CPI observations
observations = [
    (Date(2023, 1, 31), 300.0),
    (Date(2023, 2, 28), 303.0),
    (Date(2023, 3, 31), 306.0),
    (Date(2023, 4, 30), 309.0),
    (Date(2023, 5, 31), 312.0),
    (Date(2023, 6, 30), 315.0),
]

# Create US CPI index with standard settings
cpi = InflationIndex(
    "US-CPI-U",
    observations,
    Currency("USD"),
    interpolation=InflationInterpolation.STEP,  # Standard for CPI
    lag=InflationLag.months(3),  # 3-month lag for US TIPS
)

print(f"\nCreated: {cpi}")
print(f"  ID: {cpi.id}")
print(f"  Currency: {cpi.currency}")
print(f"  Interpolation: {cpi.interpolation}")
print(f"  Lag: {cpi.lag}")

# Get date range
date_range = cpi.date_range()
if date_range:
    start, end = date_range
    print(f"  Date range: {start} to {end}")

# Get index value on a specific date
test_date = Date(2023, 3, 15)
value = cpi.value_on(test_date)
print(f"\nCPI value on {test_date}: {value:.2f}")

# Calculate index ratio for inflation adjustment
base_date = Date(2023, 1, 31)
settle_date = Date(2023, 6, 30)
ratio = cpi.ratio(base_date, settle_date)
print(f"\nIndex ratio from {base_date} to {settle_date}: {ratio:.6f}")
print(f"Implied inflation: {(ratio - 1) * 100:.2f}%")

# Compare interpolation methods
print("\n=== Interpolation Comparison ===")

# Linear interpolation
linear_cpi = InflationIndex(
    "US-CPI-LINEAR",
    observations,
    Currency("USD"),
    interpolation=InflationInterpolation.LINEAR,
)

mid_month = Date(2023, 3, 15)
step_value = cpi.value_on(mid_month)
linear_value = linear_cpi.value_on(mid_month)

print(f"Value on {mid_month}:")
print(f"  Step interpolation: {step_value:.2f}")
print(f"  Linear interpolation: {linear_value:.2f}")
print(f"  Difference: {linear_value - step_value:.2f}")

# Using the builder pattern
print("\n=== Builder Pattern Example ===")

from finstack.market_data import InflationIndexBuilder

builder = InflationIndexBuilder("UK-RPI", Currency("GBP"))
builder.add_observation(Date(2023, 1, 31), 348.7)
builder.add_observation(Date(2023, 2, 28), 351.2)
builder.add_observation(Date(2023, 3, 31), 352.6)
builder.with_interpolation(InflationInterpolation.LINEAR)
builder.with_lag(InflationLag.months(2))  # 2-month lag for UK ILBs

uk_rpi = builder.build()
print(f"Created: {uk_rpi}")
print(f"  Lag: {uk_rpi.lag}")

from finstack import Date
from finstack.dates import (
    PeriodId, build_periods,
    IndexSeries, IndexInterpolation, IndexLag
)

# Period generation for reporting
periods = build_periods("2025Q1..2028Q4", "2026Q2")
for p in periods:
    print(f"{p.id}: actual={p.is_actual}")

# Inflation calculations
# Create CPI observations
observations = [
    (Date(2023, 1, 31), 300.0),
    (Date(2023, 2, 28), 303.0),
    (Date(2023, 3, 31), 306.0),
    (Date(2023, 4, 30), 309.0),
    (Date(2023, 5, 31), 312.0),
    (Date(2023, 6, 30), 315.0),
]

# Create index series with step interpolation (default for CPI)
cpi = IndexSeries(
    "US-CPI",
    observations,
    interpolation=IndexInterpolation.Step
)

print(f"Created {cpi}")
print(f"  ID: {cpi.id}")
print(f"  Observations: {len(cpi)}")

date_range = cpi.date_range()
if date_range:
    start, end = date_range
    print(f"  Date range: {start} to {end}")

# Get value on specific date
test_date = Date(2023, 2, 15)
value = cpi.value_on(test_date)
print(f"\nCPI value on {test_date}: {value:.2f}")

# Calculate index ratio for inflation adjustment
base_date = Date(2023, 1, 31)
settle_date = Date(2023, 6, 30)
ratio = cpi.ratio(base_date, settle_date)
print(f"\nIndex ratio from {base_date} to {settle_date}: {ratio:.6f}")
print(f"Implied inflation: {(ratio - 1) * 100:.2f}%")

# Test with lag
cpi_with_lag = IndexSeries(
    "US-CPI-3M-LAG",
    observations,
    interpolation=IndexInterpolation.Linear,
    lag=IndexLag.months(3)
)

print(f"\n{cpi_with_lag} with 3-month lag")

# Test lag types
lag_months = IndexLag.months(3)
lag_days = IndexLag.days(90)
no_lag = IndexLag.none()

print(f"\nLag types:")
print(f"  {lag_months}")
print(f"  {lag_days}")
print(f"  {no_lag}")

# Linear interpolation example
linear_cpi = IndexSeries(
    "US-CPI-LINEAR",
    observations,
    interpolation=IndexInterpolation.Linear
)

mid_month = Date(2023, 3, 15)
step_value = cpi.value_on(mid_month)
linear_value = linear_cpi.value_on(mid_month)

print(f"\nInterpolation comparison for {mid_month}:")
print(f"  Step interpolation: {step_value:.2f}")
print(f"  Linear interpolation: {linear_value:.2f}")

print()
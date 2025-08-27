#!/usr/bin/env python3
"""
Example: Using the new InflationIndex API for CPI/RPI calculations.

This example demonstrates how to:
1. Create inflation indices from historical observations
2. Use different interpolation methods
3. Apply lag policies for bond calculations
4. Calculate index ratios for inflation adjustments
"""

from typing import List, Tuple
from finstack import Date, Currency
from finstack.market_data import (
    InflationIndex, 
    InflationIndexBuilder,
    InflationInterpolation, 
    InflationLag
)


def create_us_cpi() -> InflationIndex:
    """Create a US CPI index with historical data."""
    # Historical CPI observations (monthly)
    observations: List[Tuple[Date, float]] = [
        (Date(2023, 1, 31), 299.170),
        (Date(2023, 2, 28), 300.840),
        (Date(2023, 3, 31), 301.836),
        (Date(2023, 4, 30), 302.918),
        (Date(2023, 5, 31), 303.294),
        (Date(2023, 6, 30), 303.841),
        (Date(2023, 7, 31), 304.348),
        (Date(2023, 8, 31), 305.537),
        (Date(2023, 9, 30), 306.269),
        (Date(2023, 10, 31), 306.139),
        (Date(2023, 11, 30), 306.060),
        (Date(2023, 12, 31), 306.746),
    ]
    
    # Create index with step interpolation (standard for CPI)
    cpi = InflationIndex(
        "US-CPI-U",
        observations,
        Currency("USD"),
        interpolation=InflationInterpolation.STEP,
        lag=InflationLag.months(3)  # 3-month lag for US TIPS
    )
    
    return cpi


def create_uk_rpi_builder() -> InflationIndex:
    """Create UK RPI using the builder pattern."""
    builder = InflationIndexBuilder("UK-RPI", Currency("GBP"))
    
    # Add observations incrementally
    builder.add_observation(Date(2023, 1, 31), 348.7)
    builder.add_observation(Date(2023, 2, 28), 351.2)
    builder.add_observation(Date(2023, 3, 31), 352.6)
    builder.add_observation(Date(2023, 4, 30), 354.0)
    builder.add_observation(Date(2023, 5, 31), 355.3)
    builder.add_observation(Date(2023, 6, 30), 356.2)
    
    # Configure interpolation and lag
    builder.with_interpolation(InflationInterpolation.LINEAR)
    builder.with_lag(InflationLag.months(2))  # 2-month lag for UK ILBs
    
    return builder.build()


def demonstrate_interpolation():
    """Compare different interpolation methods."""
    print("\n=== Interpolation Methods Comparison ===")
    
    observations = [
        (Date(2023, 1, 31), 300.0),
        (Date(2023, 2, 28), 303.0),
        (Date(2023, 3, 31), 306.0),
    ]
    
    # Step interpolation
    step_index = InflationIndex(
        "CPI-STEP",
        observations,
        Currency("USD"),
        interpolation=InflationInterpolation.STEP
    )
    
    # Linear interpolation
    linear_index = InflationIndex(
        "CPI-LINEAR",
        observations,
        Currency("USD"),
        interpolation=InflationInterpolation.LINEAR
    )
    
    # Test date between observations
    test_date = Date(2023, 2, 15)
    
    print(f"Test date: {test_date}")
    print(f"Step interpolation value: {step_index.value_on(test_date):.2f}")
    print(f"Linear interpolation value: {linear_index.value_on(test_date):.2f}")


def calculate_tips_inflation_adjustment(cpi: InflationIndex):
    """Calculate inflation adjustment for a TIPS bond."""
    print("\n=== TIPS Inflation Adjustment ===")
    
    # Bond details
    issue_date = Date(2023, 1, 15)
    settlement_date = Date(2023, 10, 15)
    
    # Get index values (with 3-month lag built in)
    base_index = cpi.value_on(issue_date)
    settlement_index = cpi.value_on(settlement_date)
    
    # Calculate index ratio
    index_ratio = cpi.ratio(issue_date, settlement_date)
    
    print(f"Issue date: {issue_date}")
    print(f"Settlement date: {settlement_date}")
    print(f"Base index (with lag): {base_index:.3f}")
    print(f"Settlement index (with lag): {settlement_index:.3f}")
    print(f"Index ratio: {index_ratio:.6f}")
    
    # Example: $1,000,000 principal
    principal = 1_000_000
    adjusted_principal = principal * index_ratio
    inflation_adjustment = adjusted_principal - principal
    
    print(f"\nOriginal principal: ${principal:,.2f}")
    print(f"Adjusted principal: ${adjusted_principal:,.2f}")
    print(f"Inflation adjustment: ${inflation_adjustment:,.2f}")


def compare_lag_policies(observations: List[Tuple[Date, float]]):
    """Compare different lag policies."""
    print("\n=== Lag Policy Comparison ===")
    
    test_date = Date(2023, 6, 15)
    
    # No lag
    no_lag = InflationIndex(
        "CPI-NO-LAG",
        observations,
        Currency("USD"),
        lag=InflationLag.none()
    )
    
    # 2-month lag
    lag_2m = InflationIndex(
        "CPI-2M-LAG",
        observations,
        Currency("USD"),
        lag=InflationLag.months(2)
    )
    
    # 3-month lag
    lag_3m = InflationIndex(
        "CPI-3M-LAG",
        observations,
        Currency("USD"),
        lag=InflationLag.months(3)
    )
    
    # 90-day lag
    lag_90d = InflationIndex(
        "CPI-90D-LAG",
        observations,
        Currency("USD"),
        lag=InflationLag.days(90)
    )
    
    print(f"Test date: {test_date}")
    print(f"No lag: {no_lag.value_on(test_date):.3f}")
    print(f"2-month lag: {lag_2m.value_on(test_date):.3f}")
    print(f"3-month lag: {lag_3m.value_on(test_date):.3f}")
    print(f"90-day lag: {lag_90d.value_on(test_date):.3f}")


def calculate_inflation_rates(cpi: InflationIndex):
    """Calculate various inflation rates."""
    print("\n=== Inflation Rate Calculations ===")
    
    # Get date range
    date_range = cpi.date_range()
    if date_range:
        start, end = date_range
        print(f"Index data range: {start} to {end}")
    
    # Calculate quarterly inflation
    q1_start = Date(2023, 1, 1)
    q1_end = Date(2023, 3, 31)
    q1_ratio = cpi.ratio(q1_start, q1_end)
    q1_rate = (q1_ratio - 1.0) * 100
    
    print(f"\nQ1 2023 inflation: {q1_rate:.2f}%")
    
    # Calculate semi-annual inflation
    h1_start = Date(2023, 1, 1)
    h1_end = Date(2023, 6, 30)
    h1_ratio = cpi.ratio(h1_start, h1_end)
    h1_rate = (h1_ratio - 1.0) * 100
    
    print(f"H1 2023 inflation: {h1_rate:.2f}%")
    
    # Calculate year-to-date inflation
    ytd_start = Date(2023, 1, 1)
    ytd_end = Date(2023, 11, 30)
    ytd_ratio = cpi.ratio(ytd_start, ytd_end)
    ytd_rate = (ytd_ratio - 1.0) * 100
    
    print(f"YTD inflation (through Nov): {ytd_rate:.2f}%")
    
    # Annualized rate
    days = (ytd_end - ytd_start)
    annualized_rate = ((ytd_ratio ** (365 / days)) - 1.0) * 100
    print(f"Annualized inflation rate: {annualized_rate:.2f}%")


def main():
    """Main example execution."""
    print("=" * 60)
    print("InflationIndex Example - Historical CPI/RPI Calculations")
    print("=" * 60)
    
    # Create US CPI index
    us_cpi = create_us_cpi()
    print(f"\nCreated: {us_cpi}")
    print(f"  Currency: {us_cpi.currency}")
    print(f"  Interpolation: {us_cpi.interpolation}")
    print(f"  Lag policy: {us_cpi.lag}")
    
    # Create UK RPI using builder
    uk_rpi = create_uk_rpi_builder()
    print(f"\nCreated: {uk_rpi}")
    print(f"  Currency: {uk_rpi.currency}")
    print(f"  Interpolation: {uk_rpi.interpolation}")
    print(f"  Lag policy: {uk_rpi.lag}")
    
    # Demonstrate different features
    demonstrate_interpolation()
    calculate_tips_inflation_adjustment(us_cpi)
    
    # Use the US CPI observations for lag comparison
    us_observations = [
        (Date(2023, 1, 31), 299.170),
        (Date(2023, 2, 28), 300.840),
        (Date(2023, 3, 31), 301.836),
        (Date(2023, 4, 30), 302.918),
        (Date(2023, 5, 31), 303.294),
        (Date(2023, 6, 30), 303.841),
    ]
    compare_lag_policies(us_observations)
    
    calculate_inflation_rates(us_cpi)
    
    print("\n" + "=" * 60)
    print("Example completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Example demonstrating the periods functionality in finstack.

This example shows how to:
- Create period IDs for different frequencies (quarterly, monthly, weekly, semi-annual, annual)
- Parse period IDs from strings
- Build period sequences with ranges and actual/forecast flags
- Work with period dates and properties
"""

from datetime import date
from finstack import PeriodId, Period, build_periods
from finstack.dates import Date


def basic_period_ids():
    """Demonstrate creating and working with PeriodId objects."""
    print("=" * 60)
    print("Basic Period IDs")
    print("=" * 60)

    # Create period IDs for different frequencies
    q1_2025 = PeriodId.quarter(2025, 1)
    print(f"Q1 2025: {q1_2025}")
    print(f"  Year: {q1_2025.year}")
    print(f"  Index: {q1_2025.index}")
    print(f"  Frequency: {q1_2025.frequency}")
    print()

    m3_2025 = PeriodId.month(2025, 3)
    print(f"March 2025: {m3_2025}")
    print(f"  Year: {m3_2025.year}")
    print(f"  Index: {m3_2025.index}")
    print(f"  Frequency: {m3_2025.frequency}")
    print()

    w10_2025 = PeriodId.week(2025, 10)
    print(f"Week 10 of 2025: {w10_2025}")
    print(f"  Frequency: {w10_2025.frequency}")
    print()

    h1_2025 = PeriodId.half(2025, 1)
    print(f"H1 2025: {h1_2025}")
    print(f"  Frequency: {h1_2025.frequency}")
    print()

    year_2025 = PeriodId.annual(2025)
    print(f"Year 2025: {year_2025}")
    print(f"  Frequency: {year_2025.frequency}")
    print()


def parsing_period_ids():
    """Demonstrate parsing period IDs from strings."""
    print("=" * 60)
    print("Parsing Period IDs from Strings")
    print("=" * 60)

    # Parse various period ID formats
    period_strings = [
        "2025Q1",  # Quarterly
        "2025Q4",
        "2025M01",  # Monthly
        "2025M12",
        "2025W01",  # Weekly
        "2025W52",
        "2025H1",  # Semi-annual
        "2025H2",
        "2025",  # Annual
    ]

    for period_str in period_strings:
        period_id = PeriodId(period_str)
        print(
            f"{period_str:10} -> Year: {period_id.year:4}, Index: {period_id.index:2}, Frequency: {period_id.frequency}"
        )
    print()


def quarterly_periods_with_actuals():
    """Demonstrate building quarterly periods with actual/forecast split."""
    print("=" * 60)
    print("Quarterly Periods with Actuals/Forecast Split")
    print("=" * 60)

    # Build periods from Q1 2024 to Q4 2025, with actuals up to Q2 2024
    periods = build_periods("2024Q1..2025Q4", "2024Q2")

    print("Period Range: 2024Q1 to 2025Q4")
    print("Actuals Until: 2024Q2")
    print()

    for period in periods:
        status = "ACTUAL  " if period.is_actual else "FORECAST"
        print(f"{period.id} ({status}): {period.start} to {period.end}")
    print()

    # Count actuals vs forecast
    actual_count = sum(1 for p in periods if p.is_actual)
    forecast_count = len(periods) - actual_count
    print(f"Total periods: {len(periods)}")
    print(f"Actual periods: {actual_count}")
    print(f"Forecast periods: {forecast_count}")
    print()


def monthly_periods_across_years():
    """Demonstrate building monthly periods across year boundaries."""
    print("=" * 60)
    print("Monthly Periods Across Year Boundaries")
    print("=" * 60)

    # Build periods from November 2024 to March 2025
    periods = build_periods("2024M11..2025M03", "2024M12")

    print("Period Range: November 2024 to March 2025")
    print("Actuals Until: December 2024")
    print()

    for period in periods:
        status = "ACTUAL  " if period.is_actual else "FORECAST"
        # Format dates nicely
        start_str = (
            f"{period.start.year}-{period.start.month:02d}-{period.start.day:02d}"
        )
        end_str = f"{period.end.year}-{period.end.month:02d}-{period.end.day:02d}"
        print(f"{str(period.id):8} ({status}): {start_str} to {end_str}")
    print()


def relative_vs_absolute_ranges():
    """Demonstrate relative and absolute period range syntax."""
    print("=" * 60)
    print("Relative vs Absolute Period Ranges")
    print("=" * 60)

    # Relative range (within same year)
    print("Relative range: 2025Q1..Q3 (end quarter relative to start year)")
    periods_relative = build_periods("2025Q1..Q3", None)
    for period in periods_relative:
        print(f"  {period.id}")
    print()

    # Absolute range (across years)
    print("Absolute range: 2024Q4..2025Q2 (full year specified for end)")
    periods_absolute = build_periods("2024Q4..2025Q2", None)
    for period in periods_absolute:
        print(f"  {period.id}")
    print()


def different_frequencies():
    """Demonstrate building periods with different frequencies."""
    print("=" * 60)
    print("Different Frequency Periods")
    print("=" * 60)

    # Weekly periods
    print("Weekly periods (first 4 weeks of 2025):")
    weekly_periods = build_periods("2025W01..W04", None)
    for period in weekly_periods:
        print(f"  {period.id}: {period.start} to {period.end}")
    print()

    # Semi-annual periods
    print("Semi-annual periods (2024-2025):")
    semi_annual_periods = build_periods("2024H1..2025H2", "2024H2")
    for period in semi_annual_periods:
        status = "ACTUAL  " if period.is_actual else "FORECAST"
        print(f"  {period.id} ({status}): {period.start} to {period.end}")
    print()

    # Annual periods
    print("Annual periods (2023-2026):")
    annual_periods = build_periods("2023..2026", "2024")
    for period in annual_periods:
        status = "ACTUAL  " if period.is_actual else "FORECAST"
        print(f"  {period.id} ({status}): {period.start} to {period.end}")
    print()


def financial_modeling_example():
    """Example showing how periods can be used in financial modeling."""
    print("=" * 60)
    print("Financial Modeling with Periods")
    print("=" * 60)

    # Build quarterly periods for a 2-year forecast with 1 year of actuals
    periods = build_periods("2024Q1..2025Q4", "2024Q4")

    print("Revenue Forecast by Quarter")
    print("-" * 40)

    # Simulate revenue data
    base_revenue = 1_000_000
    growth_rate_actual = 0.03  # 3% quarterly growth for actuals
    growth_rate_forecast = 0.05  # 5% quarterly growth for forecast

    revenues = []
    current_revenue = base_revenue

    for i, period in enumerate(periods):
        if period.is_actual:
            # For actuals, use historical growth rate
            if i > 0:
                current_revenue *= 1 + growth_rate_actual
        else:
            # For forecast, use projected growth rate
            current_revenue *= 1 + growth_rate_forecast

        revenues.append(current_revenue)

        status = "Actual" if period.is_actual else "Forecast"
        print(f"{period.id} ({status:8}): ${current_revenue:,.0f}")

    print("-" * 40)
    total_actual = sum(r for r, p in zip(revenues, periods) if p.is_actual)
    total_forecast = sum(r for r, p in zip(revenues, periods) if not p.is_actual)
    print(f"Total Actual Revenue:   ${total_actual:,.0f}")
    print(f"Total Forecast Revenue: ${total_forecast:,.0f}")
    print(f"Total Revenue:          ${sum(revenues):,.0f}")
    print()


def working_with_period_dates():
    """Demonstrate working with period start and end dates."""
    print("=" * 60)
    print("Working with Period Dates")
    print("=" * 60)

    # Build some quarterly periods
    periods = build_periods("2025Q1..Q2", None)

    for period in periods:
        # Access period dates
        start = period.start
        end = period.end

        # Calculate number of days in period
        # Note: end date is exclusive
        days_in_period = end - start  # Returns the difference in days

        print(f"{period.id}:")
        print(f"  Start: {start}")
        print(f"  End:   {end}")
        print(f"  Days in period: {days_in_period}")

        # Check if period contains specific date
        check_date = Date(2025, 2, 15)
        contains_date = start <= check_date and check_date < end
        print(f"  Contains 2025-02-15? {contains_date}")
        print()


def main():
    """Run all examples."""
    basic_period_ids()
    parsing_period_ids()
    quarterly_periods_with_actuals()
    monthly_periods_across_years()
    relative_vs_absolute_ranges()
    different_frequencies()
    financial_modeling_example()
    working_with_period_dates()


if __name__ == "__main__":
    main()

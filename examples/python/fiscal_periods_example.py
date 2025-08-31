#!/usr/bin/env python3
"""
Example demonstrating fiscal year periods functionality in finstack.

This example shows how to:
- Create fiscal year configurations for different organizations
- Build fiscal periods with custom start dates
- Compare calendar vs fiscal period dates
- Use predefined fiscal year configurations (US Federal, UK, Japan)
"""

from finstack import (
    PeriodId,
    Period,
    FiscalConfig,
    build_periods,
    build_fiscal_periods,
    Date,
)


def compare_calendar_vs_fiscal_quarters():
    """Compare calendar year quarters with fiscal year quarters."""
    print("=" * 70)
    print("Calendar Year vs Fiscal Year Quarters Comparison")
    print("=" * 70)

    # Calendar year periods
    calendar_periods = build_periods("2025Q1..Q4", None)

    # US Federal fiscal year (October 1)
    us_federal_config = FiscalConfig.us_federal()
    us_fiscal_periods = build_fiscal_periods("2025Q1..Q4", us_federal_config, None)

    # Japanese fiscal year (April 1)
    japan_config = FiscalConfig.japan()
    japan_fiscal_periods = build_fiscal_periods("2025Q1..Q4", japan_config, None)

    print("\n2025 Quarter Comparison:")
    print("-" * 70)
    print(
        f"{'Quarter':<10} {'Calendar Year':<30} {'US Federal FY':<30} {'Japan FY':<30}"
    )
    print("-" * 70)

    for i in range(4):
        cal = calendar_periods[i]
        us = us_fiscal_periods[i]
        jp = japan_fiscal_periods[i]

        cal_dates = f"{cal.start} to {cal.end}"
        us_dates = f"{us.start} to {us.end}"
        jp_dates = f"{jp.start} to {jp.end}"

        print(f"{str(cal.id):<10} {cal_dates:<30} {us_dates:<30} {jp_dates:<30}")
    print()


def us_federal_fiscal_year_example():
    """Demonstrate US Federal fiscal year (October 1 start)."""
    print("=" * 70)
    print("US Federal Fiscal Year Example")
    print("=" * 70)

    config = FiscalConfig.us_federal()
    print(
        f"US Federal fiscal year starts: Month {config.start_month}, Day {config.start_day}"
    )
    print()

    # Build fiscal quarters for FY2025 with actuals through Q2
    periods = build_fiscal_periods("2025Q1..Q4", config, "2025Q2")

    print("FY2025 Quarterly Periods:")
    print("-" * 50)

    for period in periods:
        status = "ACTUAL" if period.is_actual else "FORECAST"
        print(f"FY{period.id} ({status:8}): {period.start} to {period.end}")

    print()
    print("Key Insight: FY2025 starts in October 2024, not January 2025!")
    print()


def uk_fiscal_year_example():
    """Demonstrate UK fiscal year (April 6 start)."""
    print("=" * 70)
    print("UK Fiscal Year Example")
    print("=" * 70)

    config = FiscalConfig.uk()
    print(f"UK fiscal year starts: April {config.start_day}")
    print()

    # Build monthly periods for first quarter of UK fiscal year
    periods = build_fiscal_periods("2025M01..M03", config, None)

    print("FY2025 First Quarter Monthly Periods:")
    print("-" * 50)

    for period in periods:
        print(f"FY{period.id}: {period.start} to {period.end}")

    print()
    print("Note: UK FY2025 starts on April 6, 2024")
    print()


def japanese_fiscal_year_example():
    """Demonstrate Japanese fiscal year (April 1 start)."""
    print("=" * 70)
    print("Japanese Fiscal Year Example")
    print("=" * 70)

    config = FiscalConfig.japan()
    print(f"Japanese fiscal year starts: April {config.start_day}")
    print()

    # Build semi-annual periods for Japanese fiscal year
    periods = build_fiscal_periods("2025H1..2026H2", config, "2025H2")

    print("FY2025-2026 Semi-Annual Periods:")
    print("-" * 50)

    for period in periods:
        status = "ACTUAL" if period.is_actual else "FORECAST"
        fiscal_year = period.id.year
        print(f"FY{period.id} ({status:8}): {period.start} to {period.end}")

    print()


def custom_fiscal_year_example():
    """Demonstrate custom fiscal year configuration."""
    print("=" * 70)
    print("Custom Fiscal Year Example (July 1 start)")
    print("=" * 70)

    # Create a custom fiscal year starting July 1
    config = FiscalConfig(7, 1)  # July 1
    print(f"Custom fiscal year starts: July {config.start_day}")
    print()

    # Build annual periods with custom fiscal year
    periods = build_fiscal_periods("2024..2026", config, "2024")

    print("Custom Fiscal Years:")
    print("-" * 50)

    for period in periods:
        status = "ACTUAL" if period.is_actual else "FORECAST"
        print(f"FY{period.id} ({status:8}): {period.start} to {period.end}")

    print()
    print("Note: FY2025 runs from July 1, 2024 to June 30, 2025")
    print()


def fiscal_budget_planning_example():
    """Example showing fiscal periods for budget planning."""
    print("=" * 70)
    print("Budget Planning with Fiscal Periods")
    print("=" * 70)

    # US Federal budget planning
    config = FiscalConfig.us_federal()

    # Build quarters for FY2025 and FY2026
    periods = build_fiscal_periods("2025Q1..2026Q4", config, "2025Q1")

    print("Federal Budget Quarterly Allocations (in millions):")
    print("-" * 60)
    print(f"{'Fiscal Quarter':<15} {'Period':<25} {'Budget':<12} {'Status':<10}")
    print("-" * 60)

    base_budget = 100.0  # $100M base quarterly budget
    inflation_rate = 0.03  # 3% annual inflation

    for i, period in enumerate(periods):
        # Calculate budget with inflation
        years_from_start = i / 4.0
        budget = base_budget * (1 + inflation_rate) ** years_from_start

        status = "Actual" if period.is_actual else "Planned"
        period_str = f"{period.start} to {period.end}"

        print(
            f"FY{str(period.id):<13} {period_str:<25} ${budget:>10.2f}M  {status:<10}"
        )

    print("-" * 60)
    total_budget = sum(
        base_budget * (1 + inflation_rate) ** (i / 4.0) for i in range(len(periods))
    )
    print(f"{'Total Budget':<41} ${total_budget:>10.2f}M")
    print()


def fiscal_vs_calendar_reporting():
    """Show how to handle both fiscal and calendar reporting."""
    print("=" * 70)
    print("Dual Reporting: Fiscal and Calendar Years")
    print("=" * 70)

    # Company with July 1 fiscal year
    fiscal_config = FiscalConfig(7, 1)

    # Build fiscal quarters
    fiscal_periods = build_fiscal_periods("2025Q1..Q4", fiscal_config, "2025Q2")

    # Build calendar quarters for the same time span
    # Note: FY2025 Q1-Q4 spans from July 2024 to June 2025
    # So we need calendar periods from 2024Q3 to 2025Q2
    calendar_periods = build_periods("2024Q3..2025Q2", "2024Q4")

    print("Quarterly Revenue Report (in thousands):")
    print("-" * 70)
    print(
        f"{'Fiscal Quarter':<15} {'Calendar Coverage':<20} {'Revenue':<12} {'Status':<10}"
    )
    print("-" * 70)

    # Simulated quarterly revenues
    revenues = [450, 480, 520, 510]

    for i, (fiscal, calendar) in enumerate(zip(fiscal_periods, calendar_periods)):
        revenue = revenues[i]
        status = "Actual" if fiscal.is_actual else "Forecast"

        # Show which calendar quarters the fiscal quarter spans
        cal_coverage = f"{calendar.id}"

        print(
            f"FY2025{fiscal.id.index:5} {cal_coverage:<20} ${revenue:>10}K  {status:<10}"
        )

    print("-" * 70)
    actual_revenue = sum(
        revenues[i] for i, p in enumerate(fiscal_periods) if p.is_actual
    )
    forecast_revenue = sum(
        revenues[i] for i, p in enumerate(fiscal_periods) if not p.is_actual
    )
    print(f"{'Actual Revenue':<36} ${actual_revenue:>10}K")
    print(f"{'Forecast Revenue':<36} ${forecast_revenue:>10}K")
    print(f"{'Total FY2025':<36} ${sum(revenues):>10}K")
    print()


def fiscal_week_reporting():
    """Demonstrate weekly fiscal periods."""
    print("=" * 70)
    print("Weekly Fiscal Period Reporting")
    print("=" * 70)

    # Retail company with February 1 fiscal year (common in retail)
    config = FiscalConfig(2, 1)

    # Build first 8 weeks of fiscal year
    periods = build_fiscal_periods("2025W01..W08", config, "2025W04")

    print("Weekly Sales Report (FY2025):")
    print("-" * 60)
    print(f"{'Week':<10} {'Period':<25} {'Sales':<12} {'Status':<10}")
    print("-" * 60)

    # Simulated weekly sales with seasonality
    base_sales = 100_000

    for i, period in enumerate(periods, 1):
        # Add some variation to sales
        weekly_factor = 1.0 + (0.1 * ((i - 4) / 4))  # Gradual increase
        sales = base_sales * weekly_factor

        status = "Actual" if period.is_actual else "Forecast"
        period_str = f"{period.start} to {period.end}"

        print(f"W{i:02d}        {period_str:<25} ${sales:>10,.0f}  {status:<10}")

    print()


def main():
    """Run all fiscal period examples."""
    compare_calendar_vs_fiscal_quarters()
    us_federal_fiscal_year_example()
    uk_fiscal_year_example()
    japanese_fiscal_year_example()
    custom_fiscal_year_example()
    fiscal_budget_planning_example()
    fiscal_vs_calendar_reporting()
    fiscal_week_reporting()


if __name__ == "__main__":
    main()

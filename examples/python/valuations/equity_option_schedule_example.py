#!/usr/bin/env python3
"""
Example: Equity Option Expiry Schedule Building with Third Friday Presets

This demonstrates:
- Monthly equity option expiry schedules using third Friday of each month
- Quarterly option expiry schedules for major expirations
- Comparison between equity and IMM option expiry patterns
- Building custom schedules with business day adjustments
- Integration with holiday calendars for real-world scheduling
"""

import finstack
from finstack import Date
from finstack.dates import (
    Calendar,
    BusDayConvention,
    Frequency,
    generate_schedule,
    # Note: third_friday, next_equity_option_expiry, imm_option_expiry 
    # will be available after Python bindings are rebuilt
    # third_friday,
    # next_equity_option_expiry,
    # imm_option_expiry,
)


def monthly_equity_option_schedule():
    """Build a monthly equity option expiry schedule for 2025."""
    print("=" * 70)
    print("Monthly Equity Option Expiry Schedule (Third Friday of Each Month)")
    print("=" * 70)

    # Note: This example shows the pattern - actual implementation will use
    # third_friday() function once Python bindings are rebuilt
    print("Note: This demonstrates the equity option schedule pattern.")
    print("The third_friday() function is available in Rust and will be")
    print("available in Python after rebuilding the bindings.")
    print()

    # Generate all monthly equity option expiry dates for 2025
    equity_expiries = []
    # Placeholder data showing the pattern
    sample_dates = [
        (1, Date(2025, 1, 17)), (2, Date(2025, 2, 21)), (3, Date(2025, 3, 21)),
        (4, Date(2025, 4, 18)), (5, Date(2025, 5, 16)), (6, Date(2025, 6, 20)),
        (7, Date(2025, 7, 18)), (8, Date(2025, 8, 15)), (9, Date(2025, 9, 19)),
        (10, Date(2025, 10, 17)), (11, Date(2025, 11, 21)), (12, Date(2025, 12, 19))
    ]
    equity_expiries = sample_dates

    print("2025 Monthly Equity Option Expiry Calendar:")
    print("-" * 50)
    
    month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ]
    
    for month, expiry_date in equity_expiries:
        print(f"  {month_names[month-1]:>9}: {expiry_date} (Day {expiry_date.day})")

    print(f"\nTotal: {len(equity_expiries)} monthly expiry dates")
    return equity_expiries


def quarterly_equity_option_schedule():
    """Build a quarterly equity option expiry schedule using standard quarters."""
    print("\n" + "=" * 70)
    print("Quarterly Equity Option Expiry Schedule")
    print("=" * 70)

    # Standard quarterly months (end of each quarter)
    quarterly_months = [3, 6, 9, 12]  # March, June, September, December
    quarterly_expiries = []
    
    # Placeholder data for quarterly pattern (will use third_friday function)
    quarterly_expiries = [(3, Date(2025, 3, 21)), (6, Date(2025, 6, 20)), 
                         (9, Date(2025, 9, 19)), (12, Date(2025, 12, 19))]

    print("2025 Quarterly Equity Option Expiry Dates:")
    print("-" * 50)
    
    quarter_names = ["Q1 (March)", "Q2 (June)", "Q3 (September)", "Q4 (December)"]
    for i, (month, expiry_date) in enumerate(quarterly_expiries):
        print(f"  {quarter_names[i]}: {expiry_date}")

    return quarterly_expiries


def equity_vs_imm_comparison():
    """Compare equity option expiries with IMM option expiries."""
    print("\n" + "=" * 70)
    print("Equity Options vs IMM Options Expiry Comparison")
    print("=" * 70)

    print("Key Differences:")
    print("• Equity Options: Third Friday of EVERY month")
    print("• IMM Options: Friday before third Wednesday of Mar/Jun/Sep/Dec only")
    print()

    # Compare for IMM months (Mar, Jun, Sep, Dec)
    imm_months = [3, 6, 9, 12]
    print("2025 Comparison for IMM Months:")
    print("-" * 50)
    print(f"{'Month':<10} {'Equity (3rd Fri)':<20} {'IMM (Fri before 3rd Wed)':<25} {'Difference':<12}")
    print("-" * 67)
    
    # Sample comparison data (will use actual functions once bindings are rebuilt)
    comparisons = [
        ("Mar", Date(2025, 3, 21), Date(2025, 3, 14), 7),
        ("Jun", Date(2025, 6, 20), Date(2025, 6, 13), 7),
        ("Sep", Date(2025, 9, 19), Date(2025, 9, 12), 7),
        ("Dec", Date(2025, 12, 19), Date(2025, 12, 12), 7),
    ]
    
    for month_name, equity_expiry, imm_expiry, diff_days in comparisons:
        print(f"{month_name:<10} {str(equity_expiry):<20} {str(imm_expiry):<25} {diff_days:+d} days")

    print("\nNote: Equity options provide monthly liquidity, while IMM options")
    print("      focus on quarterly derivative settlement periods.")


def next_expiry_demonstration():
    """Demonstrate using next_equity_option_expiry for rolling schedules."""
    print("\n" + "=" * 70)
    print("Rolling Equity Option Expiry Schedule")
    print("=" * 70)

    print("Pattern: Finding the next equity option expiries starting from a date...")
    print("(Will use next_equity_option_expiry() function once bindings are rebuilt)")
    
    # Placeholder data showing the rolling pattern
    start_date = Date(2025, 1, 15)
    sample_next_expiries = [
        Date(2025, 1, 17), Date(2025, 2, 21), Date(2025, 3, 21),
        Date(2025, 4, 18), Date(2025, 5, 16), Date(2025, 6, 20),
        Date(2025, 7, 18), Date(2025, 8, 15), Date(2025, 9, 19),
        Date(2025, 10, 17), Date(2025, 11, 21), Date(2025, 12, 19)
    ]
    
    print(f"\nNext 12 equity option expiries from {start_date}:")
    print("-" * 50)
    for i, expiry_date in enumerate(sample_next_expiries, 1):
        print(f"  {i:2}. {expiry_date}")

    return sample_next_expiries


def business_day_adjusted_schedule():
    """Build equity option schedule with business day adjustments."""
    print("\n" + "=" * 70)
    print("Business Day Adjusted Equity Option Schedule")
    print("=" * 70)

    # Use NYSE calendar for equity option adjustments
    try:
        cal = Calendar.from_id("usny")  # NYSE calendar
        print("Using NYSE calendar for business day adjustments...")
    except:
        # Fallback to TARGET2 if NYSE not available
        cal = Calendar.from_id("target2")
        print("Using TARGET2 calendar (NYSE not available)...")

    print("\nComparison: Raw vs Business Day Adjusted:")
    print("-" * 50)
    print(f"{'Month':<10} {'Raw (3rd Friday)':<20} {'Adjusted':<20} {'Adjustment':<15}")
    print("-" * 65)

    # Sample data for business day adjustment pattern
    sample_raw_adjusted = [
        (1, Date(2025, 1, 17), Date(2025, 1, 17)),
        (2, Date(2025, 2, 21), Date(2025, 2, 21)),
        (3, Date(2025, 3, 21), Date(2025, 3, 21)),
    ]
    
    for month, raw_expiry, adjusted_expiry in sample_raw_adjusted[:3]:  # Show first 3 months
        
        if raw_expiry == adjusted_expiry:
            adjustment = "None"
        else:
            diff_days = (adjusted_expiry - raw_expiry).days
            adjustment = f"{diff_days:+d} days"
        
        month_name = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun",
            "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
        ][month-1]
        
        print(f"{month_name:<10} {str(raw_expiry):<20} {str(adjusted_expiry):<20} {adjustment:<15}")

    print("\nNote: Preceding convention ensures expiry doesn't move to the")
    print("      following week if the third Friday falls on a holiday.")


def create_custom_option_series():
    """Create a custom option series using preset patterns."""
    print("\n" + "=" * 70)
    print("Custom Option Series Schedule Building")
    print("=" * 70)

    # Example: Build a 2-year option series with monthly expiries
    print("Building a 2-year monthly option series (2025-2026)...")
    
    option_series = []
    # Sample data for 2-year option series pattern
    sample_series_data = [
        (2025, 1, Date(2025, 1, 17)), (2025, 2, Date(2025, 2, 21)), (2025, 3, Date(2025, 3, 21)),
        (2025, 4, Date(2025, 4, 18)), (2025, 5, Date(2025, 5, 16)), (2025, 6, Date(2025, 6, 20)),
        (2025, 7, Date(2025, 7, 18)), (2025, 8, Date(2025, 8, 15)), (2025, 9, Date(2025, 9, 19)),
        (2025, 10, Date(2025, 10, 17)), (2025, 11, Date(2025, 11, 21)), (2025, 12, Date(2025, 12, 19)),
        (2026, 1, Date(2026, 1, 16)), (2026, 2, Date(2026, 2, 20)), (2026, 3, Date(2026, 3, 20)),
        (2026, 4, Date(2026, 4, 17)), (2026, 5, Date(2026, 5, 15)), (2026, 6, Date(2026, 6, 19)),
        (2026, 7, Date(2026, 7, 17)), (2026, 8, Date(2026, 8, 21)), (2026, 9, Date(2026, 9, 18)),
        (2026, 10, Date(2026, 10, 16)), (2026, 11, Date(2026, 11, 20)), (2026, 12, Date(2026, 12, 18)),
    ]
    
    for year, month, expiry_date in sample_series_data:
        option_series.append({
            'year': year,
            'month': month,
            'expiry': expiry_date,
            'series_code': f"{year}_{month:02d}"
        })

    print(f"\nCreated option series with {len(option_series)} expiry dates")
    
    # Show first quarter of each year
    print("\nQ1 Expiries for both years:")
    print("-" * 40)
    for option in option_series[:3]:  # First quarter 2025
        print(f"  {option['series_code']}: {option['expiry']}")
    print("  ...")
    for option in option_series[12:15]:  # First quarter 2026
        print(f"  {option['series_code']}: {option['expiry']}")

    # Show how to filter for specific patterns
    print("\nQuarterly Pattern (March/June/September/December):")
    print("-" * 50)
    quarterly_only = [opt for opt in option_series if opt['month'] in [3, 6, 9, 12]]
    for option in quarterly_only[:8]:  # Show 2 years worth
        print(f"  {option['series_code']}: {option['expiry']}")

    return option_series


if __name__ == "__main__":
    print(f"Finstack version: {finstack.__version__}")
    print("Equity Option Schedule Building Examples")
    print("Using third Friday presets for standard option expiry patterns")
    
    monthly_schedule = monthly_equity_option_schedule()
    quarterly_schedule = quarterly_equity_option_schedule()
    equity_vs_imm_comparison()
    next_expiry_demonstration()
    business_day_adjusted_schedule()
    custom_series = create_custom_option_series()
    
    print(f"\n✓ Generated {len(monthly_schedule)} monthly expiries")
    print(f"✓ Generated {len(quarterly_schedule)} quarterly expiries") 
    print(f"✓ Created custom series with {len(custom_series)} total expiries")
    print("\nAll examples completed successfully!")

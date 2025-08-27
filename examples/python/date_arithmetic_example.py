#!/usr/bin/env python3
"""
Date arithmetic example demonstrating the new addition and subtraction support
for finstack dates.

This example shows:
- Adding days to dates (Date + int)
- Subtracting days from dates (Date - int)
- Calculating difference between dates (Date - Date)
- Date comparison operators
- Business day arithmetic
"""

from finstack.dates import Date


def main():
    print("=" * 60)
    print("Date Arithmetic Example - Addition and Subtraction Support")
    print("=" * 60)
    
    # Create some dates
    christmas = Date(2023, 12, 25)  # Monday
    new_year = Date(2024, 1, 1)      # Monday
    
    print(f"\nChristmas 2023: {christmas} (Monday)")
    print(f"New Year 2024: {new_year} (Monday)")
    
    # Date arithmetic - adding days
    print("\n=== Adding Days to Dates ===")
    
    # Add 5 days to Christmas
    five_days_later = christmas + 5
    print(f"Christmas + 5 days = {five_days_later}")
    
    # Add negative days (same as subtracting)
    two_days_earlier = christmas + (-2)
    print(f"Christmas + (-2) days = {two_days_earlier} (Saturday)")
    
    # Date arithmetic - subtracting days
    print("\n=== Subtracting Days from Dates ===")
    
    # Subtract 7 days from New Year
    week_before = new_year - 7
    print(f"New Year - 7 days = {week_before} (Monday)")
    
    # Date differences
    print("\n=== Calculating Days Between Dates ===")
    
    # Days between Christmas and New Year
    days_between = new_year - christmas
    print(f"Days from Christmas to New Year: {days_between} days")
    
    # Negative difference
    days_back = christmas - new_year
    print(f"Days from New Year back to Christmas: {days_back} days")
    
    # More complex example
    print("\n=== Practical Example: Holiday Period ===")
    
    # Calculate the holiday period
    dec_23 = Date(2023, 12, 23)  # Saturday
    jan_2 = Date(2024, 1, 2)      # Tuesday
    
    holiday_days = jan_2 - dec_23
    print(f"Holiday period from Dec 23 to Jan 2: {holiday_days} days")
    
    # Working days calculation (using business days)
    print("\n=== Business Days Calculation ===")
    
    friday = Date(2023, 12, 22)
    print(f"Friday: {friday}")
    
    # Add business days
    next_business_day = friday.add_business_days(1)
    print(f"Next business day after Friday: {next_business_day} (Monday)")
    
    five_biz_days = friday.add_business_days(5)
    print(f"5 business days after Friday: {five_biz_days}")
    
    # Date comparisons
    print("\n=== Date Comparisons ===")
    
    date1 = Date(2023, 6, 15)
    date2 = Date(2023, 6, 20)
    date3 = Date(2023, 6, 15)  # Same as date1
    
    print(f"Date 1: {date1}")
    print(f"Date 2: {date2}")
    print(f"Date 3: {date3}")
    
    print(f"date1 < date2: {date1 < date2}")
    print(f"date1 > date2: {date1 > date2}")
    print(f"date1 == date3: {date1 == date3}")
    print(f"date1 <= date2: {date1 <= date2}")
    print(f"date1 >= date3: {date1 >= date3}")
    
    # Practical application: Age calculation
    print("\n=== Practical Application: Age Calculation ===")
    
    birth_date = Date(1990, 6, 15)
    today = Date(2023, 12, 25)
    
    days_old = today - birth_date
    years_approx = days_old / 365.25  # Approximate years accounting for leap years
    
    print(f"Birth date: {birth_date}")
    print(f"Today: {today}")
    print(f"Days old: {days_old:,} days")
    print(f"Approximate age: {years_approx:.1f} years")
    
    # Project timeline example
    print("\n=== Project Timeline Example ===")
    
    project_start = Date(2024, 1, 15)
    milestone1_days = 30
    milestone2_days = 60
    project_end_days = 90
    
    milestone1 = project_start + milestone1_days
    milestone2 = project_start + milestone2_days
    project_end = project_start + project_end_days
    
    print(f"Project start: {project_start}")
    print(f"Milestone 1 (30 days): {milestone1}")
    print(f"Milestone 2 (60 days): {milestone2}")
    print(f"Project end (90 days): {project_end}")
    
    # Check if we're on schedule
    current_date = Date(2024, 2, 20)
    days_elapsed = current_date - project_start
    days_remaining = project_end - current_date
    
    print(f"\nStatus check on {current_date}:")
    print(f"Days elapsed: {days_elapsed}")
    print(f"Days remaining: {days_remaining}")
    print(f"Past milestone 1: {current_date > milestone1}")
    print(f"Past milestone 2: {current_date > milestone2}")
    
    print("\n" + "=" * 60)
    print("Date arithmetic operations completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()

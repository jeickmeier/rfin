#!/usr/bin/env python3
"""
Example showcasing the new `Date` class exposed by RustFin's Python bindings.


"""

import rfin
from rfin import Date
# Explicit import for DayCount enum
from rfin.dates import DayCount, Calendar, BusDayConvention, available_calendars

print(f"RustFin version: {rfin.__version__}\n")

print("=== Date Examples ===")

# Create a valid date
trade_date = Date(2025, 6, 27)
print(f"Trade date: {trade_date} (Y={trade_date.year}, M={trade_date.month}, D={trade_date.day})")

# Equality check
same_day = Date(2025, 6, 27)
print(f"Same day equality check: {trade_date == same_day}")

# Another date
settle_date = Date(2025, 7, 1)
print(f"Settle date: {settle_date}")

# Simple comparison (lexicographic via string works for demo)
is_after = str(settle_date) > str(trade_date)
print(f"Is settle after trade? {is_after}")

# Weekend check
print(f"Is trade date a weekend? {trade_date.is_weekend()}")
print(f"Quarter of trade date: Q{trade_date.quarter()}")
print(f"Fiscal year: {trade_date.fiscal_year()}")



# Business-day addition
plus_3bd = trade_date.add_business_days(3)
minus_2bd = trade_date.add_business_days(-2)
print(f"Trade +3 business days = {plus_3bd}")
print(f"Trade -2 business days = {minus_2bd}")

# Day-count year fraction examples
convention = DayCount.act360()
year_frac = convention.year_fraction(trade_date, settle_date)
print(f"Year fraction {convention} between {trade_date} and {settle_date}: {year_frac:.6f}")

# Demonstrate invalid date handling
try:
    invalid = Date(2025, 2, 30)
except ValueError as e:
    print(f"Caught expected error for invalid date: {e}")

# List built-in holiday calendars
print("\nAvailable holiday calendars:")
for cal_id in available_calendars():
    print(f"  • {cal_id}")
    
# Calendar usage
cal = Calendar.target2()
print("\n=== Calendar (TARGET2) ===")
print(f"Is {trade_date} TARGET2 holiday? {cal.is_holiday(trade_date)}")
adj_follow = cal.adjust(trade_date, BusDayConvention.Following)
print(f"Following adjustment of {trade_date} => {adj_follow}") 
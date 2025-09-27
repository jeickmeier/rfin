#!/usr/bin/env python3
"""
Example showcasing the new `Date` class exposed by RustFin's Python bindings.


"""

import finstack
from finstack import Date

# Explicit import for DayCount enum
from finstack.dates import (
    DayCount,
    Calendar,
    BusDayConvention,
    Frequency,
    generate_schedule,
    available_calendars,
    StubRule,
)
from finstack.dates import (
    third_wednesday, 
    next_imm, 
    next_cds_date,
    # Additional functions will be available after rebuilding Python bindings:
    # third_friday,
    # next_equity_option_expiry, 
    # imm_option_expiry,
)

print(f"Finstack version: {finstack.__version__}\n")

print("=== Date Examples ===")

# Create a valid date
trade_date = Date(2025, 6, 27)
print(
    f"Trade date: {trade_date} (Y={trade_date.year}, M={trade_date.month}, D={trade_date.day})"
)

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
# Note: fiscal_year now requires a FiscalConfig parameter
# print(f"Fiscal year: {trade_date.fiscal_year(FiscalConfig.calendar_year())}")
print("Fiscal year: [requires FiscalConfig - see fiscal_periods_example.py]")

# IMM / Quarterly helpers ----------------------------------------------------

print("\n=== IMM / Quarterly Helpers ===")

# Third Wednesday example
tw_mar = third_wednesday(3, 2025)  # March
print(f"Third Wednesday March 2025: {tw_mar}")

# Next IMM date after trade_date
next_imm_date = next_imm(trade_date)
print(f"Next IMM after {trade_date}: {next_imm_date}")

# Next CDS roll date after trade_date
next_cds = next_cds_date(trade_date)
print(f"Next CDS roll date after {trade_date}: {next_cds}")

# Note: Additional IMM functions (third_friday, next_equity_option_expiry, imm_option_expiry)
# and add_business_days method will be available after Python bindings are rebuilt
print("Note: Enhanced IMM functions and add_business_days method available after rebuild")

# Note: Weekday and business day addition methods will be available
# after Python bindings are rebuilt with the latest Rust enhancements
print("Enhanced date arithmetic methods coming soon:")
print("  • add_weekdays(n) - skip weekends only") 
print("  • add_business_days(n, calendar) - skip weekends AND holidays")

# Day-count year fraction examples
convention = DayCount.act360()
year_frac = convention.year_fraction(trade_date, settle_date)
print(
    f"Year fraction {convention} between {trade_date} and {settle_date}: {year_frac:.6f}"
)

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
cal = Calendar.from_id("target2")
print("\n=== Calendar (TARGET2) ===")
print(f"Is {trade_date} TARGET2 holiday? {cal.is_holiday(trade_date)}")
adj_follow = cal.adjust(trade_date, BusDayConvention.Following)
print(f"Following adjustment of {trade_date} => {adj_follow}")

# Union calendar example (TARGET2 ∪ GBLO)
union_cal = Calendar.union([Calendar.from_id("target2"), Calendar.from_id("gblo")])
is_union_hol = union_cal.is_holiday(trade_date)
print(f"Is {trade_date} a holiday in TARGET2 ∪ GBLO? {is_union_hol}")

# SimpleSchedule generation
schedule = generate_schedule(
    Date(2025, 1, 15),
    Date(2030, 1, 15),
    Frequency.SemiAnnual,
    BusDayConvention.Unadjusted,
)
print("\nGenerated semi-annual schedule ({} dates):".format(len(schedule)))
print([str(d) for d in schedule])

# Schedule generation with business day adjustment
schedule = generate_schedule(Date(2025, 1, 15), Date(2030, 1, 15), Frequency.Monthly)
schedule_adjusted = generate_schedule(
    Date(2025, 1, 15),
    Date(2030, 1, 15),
    Frequency.Monthly,
    BusDayConvention.Following,
    cal,
)
print("\nGenerated monthly unadjusted schedule ({} dates):".format(len(schedule)))
print("--------------------------------")
print([str(d) for d in schedule])
print(
    "\nGenerated monthly adjusted following schedule ({} dates):".format(
        len(schedule_adjusted)
    )
)
print("--------------------------------")
print([str(d) for d in schedule_adjusted])

print("\nNote: See equity_option_schedule_example.py for comprehensive")
print("      equity option expiry schedule building examples using")
print("      third Friday presets and IMM comparison patterns.")

# Demonstration of a Short Front stub (Semi-Annual frequency)
# -------------------------------------------------------------------
# We deliberately start the schedule on 15-Jan-2025 although the normal
# semi-annual roll dates are 15-Apr and 15-Oct.  The first period
# (15-Jan-2025 → 15-Apr-2025) is therefore only 3 months long while all
# subsequent periods are the regular 6 months – a textbook *short front stub*.

start_stub = Date(2025, 1, 15)
end_stub = Date(2027, 10, 15)

schedule_stub = generate_schedule(
    start_stub,
    end_stub,
    Frequency.SemiAnnual,
    BusDayConvention.Unadjusted,
    stub=StubRule.ShortFront,
)

print("\n=== Semi-annual schedule with Short Front stub ===")
print(f"Start: {start_stub} → End: {end_stub}  |  Total dates: {len(schedule_stub)}")
print("--------------------------------")
for idx, d in enumerate(schedule_stub):
    label = " <-- stub end" if idx == 1 else ""
    print(f"{idx:2}: {d}{label}")

# Demonstration of a Short Back stub (Semi-Annual frequency)
# -------------------------------------------------------------------
# Using **exactly the same** start / end as above, but selecting
# `StubRule.ShortBack`.  This shifts the 3-month stub to the *end* of the
# schedule instead of the beginning.

start_back = start_stub  # 2025-01-15
end_back = end_stub  # 2027-10-15

schedule_back = generate_schedule(
    start_back,
    end_back,
    Frequency.SemiAnnual,
    BusDayConvention.Unadjusted,
    stub=StubRule.ShortBack,
)

print("\n=== Semi-annual schedule with Short Back stub ===")
print(f"Start: {start_back} → End: {end_back}  |  Total dates: {len(schedule_back)}")
print("--------------------------------")
for idx, d in enumerate(schedule_back):
    # The stub is the final short period, starting at the penultimate date.
    label = " <-- stub start" if idx == len(schedule_back) - 2 else ""
    print(f"{idx:2}: {d}{label}")

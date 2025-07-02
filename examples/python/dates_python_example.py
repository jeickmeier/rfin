#!/usr/bin/env python3
"""
Example showcasing the new `Date` class exposed by RustFin's Python bindings.


"""

import rfin
from rfin import Date
# Explicit import for DayCount enum
from rfin.dates import DayCount, Calendar, BusDayConvention, Frequency, generate_schedule, available_calendars, StubRule
from rfin.dates import third_wednesday, next_imm, next_cds_date
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

# Union calendar example (TARGET2 ∪ GBLO)
union_cal = Calendar.union([Calendar.target2(), Calendar.gblo()])
is_union_hol = union_cal.is_holiday(trade_date)
print(f"Is {trade_date} a holiday in TARGET2 ∪ GBLO? {is_union_hol}")

# SimpleSchedule generation
schedule = generate_schedule(Date(2025, 1, 15), Date(2030, 1, 15), Frequency.SemiAnnual, BusDayConvention.Unadjusted)
print("\nGenerated semi-annual schedule ({} dates):".format(len(schedule)))
print([str(d) for d in schedule])

# Schedule generation with business day adjustment
schedule = generate_schedule(Date(2025, 1, 15), Date(2030, 1, 15), Frequency.Monthly)
schedule_adjusted = generate_schedule(Date(2025, 1, 15), Date(2030, 1, 15), Frequency.Monthly, BusDayConvention.Following, cal)
print("\nGenerated monthly unadjusted schedule ({} dates):".format(len(schedule)))
print("--------------------------------")
print([str(d) for d in schedule])
print("\nGenerated monthly adjusted following schedule ({} dates):".format(len(schedule_adjusted)))
print("--------------------------------")
print([str(d) for d in schedule_adjusted])

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
end_back   = end_stub    # 2027-10-15

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
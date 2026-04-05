# Schedules & Periods

## Schedule Generation

The `ScheduleBuilder` generates payment date sequences using a fluent API.
Schedules are the backbone of coupon calculations, swap legs, and amortization.

### Builder API

**Rust**

```rust,no_run
use finstack_core::dates::schedule::{ScheduleBuilder, StubKind};
use finstack_core::dates::calendar::CalendarRegistry;
use finstack_core::dates::calendar::BusinessDayConvention;
use finstack_core::dates::tenor::Tenor;

let nyse = CalendarRegistry::resolve_str("NYSE").unwrap();

let schedule = ScheduleBuilder::new(
        date!(2025-01-15),   // start
        date!(2027-01-15),   // end
    )?
    .frequency(Tenor::semi_annual())
    .stub_rule(StubKind::ShortFront)
    .adjust_with(BusinessDayConvention::ModifiedFollowing, nyse)
    .end_of_month(false)
    .build()?;

for d in &schedule.dates {
    println!("{d}");
}
```

**Python**

```python
from finstack.core.dates import (
    ScheduleBuilder, Tenor, StubKind,
    BusinessDayConvention, get_calendar,
)
from datetime import date

nyse = get_calendar("NYSE")

sched = (
    ScheduleBuilder(date(2025, 1, 15), date(2027, 1, 15))
    .frequency(Tenor.semi_annual())
    .stub_rule(StubKind.SHORT_FRONT)
    .adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, nyse)
    .build()
)

for d in sched.dates:
    print(d)
```

### Stub Handling

When the period between start and end doesn't divide evenly by the frequency,
a *stub* (short or long) period is inserted:

| Stub Kind | Behavior |
|-----------|----------|
| `None` | No special handling (dates must divide evenly) |
| `ShortFront` | Short first period |
| `ShortBack` | Short last period |
| `LongFront` | Merge first two periods into one long period |
| `LongBack` | Merge last two periods into one long period |

### IMM Dates

Standard and CDS IMM date rules are supported:

```python
# Standard IMM: third Wednesday of month
sched = ScheduleBuilder(start, end).frequency(Tenor.quarterly()).imm().build()

# CDS IMM: 20th of Mar/Jun/Sep/Dec
sched = ScheduleBuilder(start, end).frequency(Tenor.quarterly()).cds_imm().build()
```

## Tenor

The `Tenor` type represents a time period by months or days:

```python
from finstack.core.dates import Tenor

Tenor.monthly()        # 1 month
Tenor.quarterly()      # 3 months
Tenor.semi_annual()    # 6 months
Tenor.annual()         # 12 months
Tenor.weekly()         # 7 days
Tenor.from_months(18)  # 18 months
Tenor.from_days(30)    # 30 days
```

## Fiscal Periods

`PeriodId` identifies a calendar period (quarter, month, half-year) and supports
navigation:

**Python**

```python
from finstack.core.dates.periods import PeriodId, build_periods

# Named constructor
q1 = PeriodId.quarter(2025, 1)
q2 = q1.next()                   # Q2 2025
q4_prev = q1.prev()              # Q4 2024

# Build a range
plan = build_periods("2025Q1..Q4")
for period in plan.periods:
    print(f"{period.id}: {period.start} → {period.end}")

# Monthly periods
months = build_periods("2025M01..M12")
```

### Fiscal Year Configuration

For non-calendar fiscal years (e.g., April start):

```python
from finstack.core.dates.periods import build_fiscal_periods, FiscalConfig

fiscal = FiscalConfig(start_month=4, start_day=1)
periods = build_fiscal_periods(fiscal)
```

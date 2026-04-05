# Dates & Calendars

## Date Types

Finstack re-exports the `time` crate's `Date` type. A safe constructor prevents
panics from invalid date inputs:

**Rust**

```rust,no_run
use finstack_core::dates::create_date;
use time::Month;

let d = create_date(2025, Month::January, 15)?;   // Ok(2025-01-15)
let bad = create_date(2025, Month::February, 30);  // Err(...)
```

**Python**

```python
from datetime import date

# Python uses stdlib datetime.date directly
d = date(2025, 1, 15)
```

## Holiday Calendars

Finstack provides holiday calendars for major financial centers. Each calendar
implements the `HolidayCalendar` trait.

### Built-In Calendars

| Code | Center | Notes |
|------|--------|-------|
| `US` | United States | Federal holidays |
| `GB` | United Kingdom | Bank holidays |
| `TARGET2` | Euro zone | ECB TARGET2 |
| `JP` | Japan | Exchange holidays |
| `NYSE` | New York Stock Exchange | Trading calendar |
| `NYFE` / `NYMEX` | US futures exchanges | Commodity calendars |

### Usage

**Rust**

```rust,no_run
use finstack_core::dates::calendar::{CalendarRegistry, HolidayCalendar};

let nyse = CalendarRegistry::resolve_str("NYSE")
    .expect("NYSE calendar not found");

assert!(!nyse.is_business_day(date!(2025-01-01)));  // New Year's Day
assert!(nyse.is_business_day(date!(2025-01-02)));   // Regular Thursday
```

**Python**

```python
from finstack.core.dates import get_calendar, available_calendar_codes
from datetime import date

# Discover available calendars
codes = available_calendar_codes()  # ["US", "GB", "NYSE", ...]

# Look up by code
nyse = get_calendar("NYSE")
assert not nyse.is_business_day(date(2025, 1, 1))  # Holiday
assert nyse.is_business_day(date(2025, 1, 2))       # Business day
```

## Business Day Conventions

When a date falls on a weekend or holiday, a `BusinessDayConvention` determines
how to adjust it:

| Convention | Rule |
|------------|------|
| `Following` | Move to next business day |
| `Preceding` | Move to previous business day |
| `ModifiedFollowing` | Following, but stay in same month (fall back to Preceding) |
| `ModifiedPreceding` | Preceding, but stay in same month (fall back to Following) |
| `Unadjusted` | No adjustment |

**Python**

```python
from finstack.core.dates import adjust, BusinessDayConvention, get_calendar
from datetime import date

nyse = get_calendar("NYSE")

# Saturday Jan 4, 2025 → adjusted to Monday Jan 6
adjusted = adjust(
    date(2025, 1, 4),
    BusinessDayConvention.MODIFIED_FOLLOWING,
    nyse,
)
```

## Day Count Conventions

Day count conventions determine how to compute year fractions between two dates.
These are fundamental to accrued interest, coupon calculations, and discounting.

| Convention | Formula | Typical Use |
|------------|---------|------------|
| `Act360` | actual days / 360 | Money markets, LIBOR/SOFR |
| `Act365F` | actual days / 365 | UK gilts, some RFRs |
| `ActAct` | ISDA actual/actual | US Treasuries |
| `ActActIsma` | ICMA actual/actual (needs frequency) | Eurobonds |
| `Thirty360` | 30/360 variants (US, EU, ISDA) | US corporate bonds |
| `ThirtyE360` | 30E/360 (Eurobond) | Eurobonds |
| `Bus252` | business days / 252 (needs calendar) | Brazilian market |

**Python**

```python
from finstack.core.dates import DayCount, DayCountContext
from datetime import date

# Simple usage
yf = DayCount.ACT_360.year_fraction(date(2025, 1, 1), date(2026, 1, 1))
# yf ≈ 1.01389 (365/360)

# Bus/252 requires a calendar context
nyse = get_calendar("NYSE")
ctx = DayCountContext(calendar=nyse)
yf_bus = DayCount.BUS_252.year_fraction(date(2025, 1, 2), date(2025, 4, 2), ctx)
```

**Rust**

```rust,no_run
use finstack_core::dates::daycount::{DayCount, DayCountCtx};

let yf = DayCount::Act360.year_fraction(
    date!(2025-01-01),
    date!(2026-01-01),
    DayCountCtx::default(),
)?;
// yf ≈ 1.01389
```

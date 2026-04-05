# Core Primitives

The `finstack-core` crate (~43K lines) provides the foundational types that every
other crate depends on: currencies, money, dates, calendars, day counts,
schedules, market data containers, and math utilities.

## Module Map

| Module | Description |
|--------|-------------|
| `currency` | ISO 4217 currency codes, decimal places, constants |
| `money` | Currency-safe arithmetic with rounding policies |
| `dates` | Date types, business day conventions |
| `calendars` | Holiday calendars, combined calendars |
| `day_count` | Act/360, Act/365, 30/360, Act/Act and variants |
| `schedule` | Payment schedule generation |
| `periods` | Fiscal period construction |
| `config` | Global configuration (rounding, scale) |
| `market_data` | Curves, surfaces, FX rates, market context |
| `math` | Interpolation, root finding, XIRR |

## Detail Pages

- [Currency & Money](currency-money.md)
- [Dates & Calendars](dates-calendars.md)
- [Schedules & Periods](schedules-periods.md)
- [Configuration](config.md)

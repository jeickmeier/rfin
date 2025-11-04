# API Reference: Python ↔ TypeScript/WASM

This page provides a comprehensive comparison of the finstack APIs across Python and TypeScript/WASM bindings.

> **Quick Tip:** Use your browser's search (Ctrl/Cmd+F) to find specific APIs.

## Core Types

### Currency

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Currency("USD")` | `new Currency("USD")` | Create currency from code |
| `Currency.from_code("USD")` | `Currency.fromCode("USD")` | Alternative constructor |
| `Currency.from_numeric(840)` | `Currency.fromNumeric(840)` | Create from numeric code |
| `Currency.all()` | `Currency.all()` | List all currencies |
| `currency.code` | `currency.code` | Currency code (getter) |
| `currency.numeric_code` | `currency.numericCode` | Numeric code (getter) |
| `currency.name` | `currency.name` | Currency name (getter) |

### Money

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Money(100.0, currency)` | `new Money(100.0, currency)` | Create money amount |
| `Money.from_code(100.0, "USD")` | `Money.fromCode(100.0, "USD")` | Create from currency code |
| `Money.from_tuple((100.0, "USD"))` | `Money.fromTuple([100.0, "USD"])` | Create from tuple/array |
| `money.amount` | `money.amount` | Amount value (getter) |
| `money.currency` | `money.currency` | Currency object (getter) |
| `money.format()` | `money.format()` | Formatted string |
| `money + other` | N/A (use `money.add(other)`) | Addition (operator overload) |
| `money.add(other)` | `money.add(other)` | Addition (method) |
| `money.subtract(other)` | `money.subtract(other)` | Subtraction |
| `money.multiply(scalar)` | `money.multiply(scalar)` | Scalar multiplication |
| `money.divide(scalar)` | `money.divide(scalar)` | Scalar division |

### Date & Time

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `date(2024, 1, 15)` (stdlib) | `new FsDate(2024, 1, 15)` | Create date |
| `add_months(date, 3)` | `addMonths(date, 3)` | Add months to date |
| `last_day_of_month(date)` | `lastDayOfMonth(date)` | Last day of month |
| `days_in_month(2024, 2)` | `daysInMonth(2024, 2)` | Days in a month |
| `is_leap_year(2024)` | `isLeapYear(2024)` | Check if leap year |
| `date_to_days_since_epoch(date)` | `dateToDaysSinceEpoch(date)` | Convert to epoch days |
| `days_since_epoch_to_date(days)` | `daysSinceEpochToDate(days)` | Convert from epoch days |

### Calendar

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `get_calendar("usny")` | `getCalendar("usny")` | Get calendar by code |
| `available_calendars()` | `availableCalendars()` | List all calendars |
| `available_calendar_codes()` | `availableCalendarCodes()` | List calendar codes |
| `calendar.is_business_day(date)` | `calendar.isBusinessDay(date)` | Check if business day |
| `calendar.is_holiday(date)` | `calendar.isHoliday(date)` | Check if holiday |
| `adjust(date, convention, cal)` | `adjust(date, convention, cal)` | Adjust date |

### Day Count Conventions

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `DayCount.ACT_360` | `DayCount.act360()` | Act/360 convention |
| `DayCount.ACT_365F` | `DayCount.act365f()` | Act/365F convention |
| `DayCount.THIRTY_360` | `DayCount.thirty360()` | 30/360 convention |
| `DayCount.ACT_ACT_ISDA` | `DayCount.actActIsda()` | Act/Act ISDA |
| `dc.year_fraction(start, end, ctx)` | `dc.yearFraction(start, end, ctx)` | Calculate year fraction |
| `dc.days_between(start, end)` | `dc.daysBetween(start, end)` | Days between dates |

### Frequencies

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Frequency.ANNUAL` | `Frequency.annual()` | Annual frequency |
| `Frequency.SEMI_ANNUAL` | `Frequency.semiAnnual()` | Semi-annual frequency |
| `Frequency.QUARTERLY` | `Frequency.quarterly()` | Quarterly frequency |
| `Frequency.MONTHLY` | `Frequency.monthly()` | Monthly frequency |
| `Frequency.WEEKLY` | `Frequency.weekly()` | Weekly frequency |

## Market Data

### Discount Curves

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `DiscountCurve(id, date, points, ...)` | `new DiscountCurve(id, date, points, ...)` | Create discount curve |
| `curve.df(time)` | `curve.df(time)` | Discount factor |
| `curve.zero_rate(time)` | `curve.zeroRate(time)` | Zero rate |
| `curve.forward_rate(t1, t2)` | `curve.forwardRate(t1, t2)` | Forward rate |
| `curve.day_count` | `curve.dayCount` | Day count convention (getter) |
| `curve.reference_date` | `curve.referenceDate` | Reference date (getter) |

### Market Context

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `MarketContext()` | `new MarketContext()` | Create market context |
| `ctx.insert_discount(curve)` | `ctx.insertDiscount(curve)` | Add discount curve |
| `ctx.insert_forward(curve)` | `ctx.insertForward(curve)` | Add forward curve |
| `ctx.insert_hazard(curve)` | `ctx.insertHazard(curve)` | Add hazard curve |
| `ctx.insert_surface(surface)` | `ctx.insertSurface(surface)` | Add vol surface |
| `ctx.insert_fx(matrix)` | `ctx.insertFx(matrix)` | Add FX matrix |
| `ctx.insert_price(id, scalar)` | `ctx.insertPrice(id, scalar)` | Add price scalar |
| `ctx.get_discount(id)` | `ctx.getDiscount(id)` | Get discount curve |
| `ctx.stats()` | `ctx.stats()` | Market data statistics |

### FX Matrix

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `FxMatrix()` | `new FxMatrix()` | Create FX matrix |
| `fx.set_quote(ccy1, ccy2, rate)` | `fx.setQuote(ccy1, ccy2, rate)` | Set exchange rate |
| `fx.rate(ccy1, ccy2, date, policy)` | `fx.rate(ccy1, ccy2, date, policy)` | Get exchange rate |
| `fx.has_rate(ccy1, ccy2)` | `fx.hasRate(ccy1, ccy2)` | Check if rate exists |

## Instruments

### Bond

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Bond.treasury(id, notional, ...)` | `Bond.treasury(id, notional, ...)` | Create treasury bond |
| `Bond.corporate(id, notional, ...)` | `Bond.corporate(id, notional, ...)` | Create corporate bond |
| `Bond.zero_coupon(id, notional, ...)` | `Bond.zeroCoupon(id, notional, ...)` | Create zero-coupon bond |
| `Bond.builder(id)` | `Bond.builder(id)` | Start builder |
| `bond.notional` | `bond.notional` | Notional amount (getter) |
| `bond.coupon_rate` | `bond.couponRate` | Coupon rate (getter) |
| `bond.maturity` | `bond.maturity` | Maturity date (getter) |

### Interest Rate Swap

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `InterestRateSwap(id, notional, ...)` | `new InterestRateSwap(id, notional, ...)` | Create IRS |
| `IRS.builder(id)` | `InterestRateSwap.builder(id)` | Start builder (Note: Alias differs!) |
| `swap.fixed_rate` | `swap.fixedRate` | Fixed rate (getter) |
| `swap.floating_spread` | `swap.floatingSpread` | Floating spread (getter) |

### Credit Default Swap

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `CreditDefaultSwap.buy_protection(...)` | `CreditDefaultSwap.buyProtection(...)` | Buy protection |
| `CreditDefaultSwap.sell_protection(...)` | `CreditDefaultSwap.sellProtection(...)` | Sell protection |
| `cds.notional` | `cds.notional` | Notional amount (getter) |
| `cds.spread_bps` | `cds.spreadBps` | Spread in bps (getter) |
| `cds.recovery_rate` | `cds.recoveryRate` | Recovery rate (getter) |

### Options

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `EquityOption(id, ...)` | `new EquityOption(id, ...)` | Create equity option |
| `FxOption(id, ...)` | `new FxOption(id, ...)` | Create FX option |
| `Swaption(id, ...)` | `new Swaption(id, ...)` | Create swaption |
| `option.strike` | `option.strike` | Strike price (getter) |
| `option.expiry` | `option.expiry` | Expiry date (getter) |
| `option.option_type` | `option.optionType` | Call or Put (getter) |

## Calibration

### Discount Curve Calibrator

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `DiscountCurveCalibrator(id, date, ccy)` | `new DiscountCurveCalibrator(id, date, ccy)` | Create calibrator |
| `cal.with_config(config)` | `cal.withConfig(config)` | Set calibration config |
| `cal.with_solver_kind(kind)` | `cal.withSolverKind(kind)` | Set solver type |
| `cal.calibrate(quotes, market)` | `cal.calibrate(quotes, market)` | Run calibration |

### Calibration Config

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `CalibrationConfig.default()` | `CalibrationConfig.default()` | Default config |
| `CalibrationConfig.multi_curve()` | `CalibrationConfig.multiCurve()` | Multi-curve config |
| `config.with_max_iterations(n)` | `config.withMaxIterations(n)` | Set max iterations |
| `config.with_tolerance(tol)` | `config.withTolerance(tol)` | Set tolerance |

### Quotes

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `RatesQuote.deposit(date, rate, dc)` | `RatesQuote.deposit(date, rate, dc)` | Deposit quote |
| `RatesQuote.swap(date, rate, ...)` | `RatesQuote.swap(date, rate, ...)` | Swap quote |
| `CreditQuote.cds(date, spread, ...)` | `CreditQuote.cds(date, spread, ...)` | CDS quote |
| `VolQuote.atm(expiry, vol)` | `VolQuote.atm(expiry, vol)` | ATM vol quote |

## Pricing & Risk

### Pricer Registry

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `PricerRegistry.standard()` | `createStandardRegistry()` | Create standard registry |
| `registry.price(instrument, ctx)` | `registry.price(instrument, ctx)` | Price instrument |
| `registry.price_with_metrics(...)` | `registry.priceWithMetrics(...)` | Price with metrics |

### Risk Functions

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `krd_dv01_ladder(bond, market, ...)` | `krdDv01Ladder(bond, market, ...)` | Key rate DV01 |
| `cs01_ladder(cds, market, ...)` | `cs01Ladder(cds, market, ...)` | Credit spread ladder |

## Statements

### Model Builder

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `ModelBuilder.new(name)` | `ModelBuilder.new(name)` | Create builder |
| `builder.periods(spec, actual)` | `builder.periods(spec, actual)` | Set periods |
| `builder.value(id, values)` | `builder.value(id, values)` | Add value node |
| `builder.compute(id, formula)` | `builder.compute(id, formula)` | Add computed node |
| `builder.forecast(id, spec)` | `builder.forecast(id, spec)` | Add forecast |
| `builder.build()` | `builder.build()` | Build model |

### Evaluator

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Evaluator.new()` | `Evaluator.new()` | Create evaluator |
| `eval.evaluate(model)` | `eval.evaluate(model)` | Evaluate model |
| `results.get(id, period)` | `results.get(id, period)` | Get result value |

### Forecast Specifications

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `ForecastSpec.fill_forward()` | `ForecastSpec.fillForward()` | Forward fill |
| `ForecastSpec.growth(rate)` | `ForecastSpec.growth(rate)` | Growth rate |
| `ForecastSpec.curve(curve)` | `ForecastSpec.curve(curve)` | Curve forecast |
| `ForecastSpec.normal(mean, std)` | `ForecastSpec.normal(mean, std)` | Normal distribution |

## Scenarios

### Scenario Engine

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `ScenarioEngine()` | `new ScenarioEngine()` | Create engine |
| `engine.apply(scenario, ctx)` | `engine.apply(scenario, ctx)` | Apply scenario |
| `engine.preview(scenario, ctx)` | `engine.preview(scenario, ctx)` | Preview scenario |

### Scenario Specification

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `ScenarioSpec(id, ops, ...)` | `new ScenarioSpec(id, ops, ...)` | Create scenario |
| `OperationSpec.curve_parallel_bp(...)` | `OperationSpec.curveParallelBp(...)` | Parallel curve shift |
| `OperationSpec.equity_shock(id, pct)` | `OperationSpec.equityShock(id, pct)` | Equity shock |
| `OperationSpec.vol_surface_shift(...)` | `OperationSpec.volSurfaceShift(...)` | Vol surface shift |

## Portfolio

### Portfolio

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `Portfolio(id, ...)` | `new Portfolio(id, ...)` | Create portfolio |
| `PortfolioBuilder(id)` | `new PortfolioBuilder(id)` | Create builder |
| `builder.name(name)` | `builder.name(name)` | Set name |
| `builder.base_ccy(ccy)` | `builder.baseCcy(ccy)` | Set base currency |
| `builder.entity(entity)` | `builder.entity(entity)` | Add entity |
| `builder.position(position)` | `builder.position(position)` | Add position |
| `builder.build()` | `builder.build()` | Build portfolio |

### Portfolio Operations

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `value_portfolio(portfolio, market)` | `valuePortfolio(portfolio, market)` | Value portfolio |
| `aggregate_by_attribute(portfolio, attr)` | `aggregateByAttribute(portfolio, attr)` | Aggregate by attribute |
| `group_by_attribute(portfolio, attr)` | `groupByAttribute(portfolio, attr)` | Group by attribute |

## Utility Functions

### Period Building

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `build_periods("2024Q1..Q4", "2024Q2")` | `buildPeriods("2024Q1..Q4", "2024Q2")` | Build periods |
| `build_fiscal_periods("FY2024Q1..Q4", ...)` | `buildFiscalPeriods("FY2024Q1..Q4", ...)` | Build fiscal periods |

### IMM Dates

| Python | TypeScript/WASM | Description |
|--------|-----------------|-------------|
| `next_imm(date)` | `nextImm(date)` | Next IMM date |
| `next_cds_date(date)` | `nextCdsDate(date)` | Next CDS date |
| `next_imm_option_expiry(...)` | `nextImmOptionExpiry(...)` | Next IMM option expiry |
| `imm_option_expiry(year, month)` | `immOptionExpiry(year, month)` | IMM option expiry |
| `third_friday(year, month)` | `thirdFriday(year, month)` | Third Friday |
| `third_wednesday(year, month)` | `thirdWednesday(year, month)` | Third Wednesday |

## Notes

### Type Differences

- **Python dates:** Use stdlib `datetime.date`
- **WASM dates:** Use `FsDate` class or JavaScript `Date` (auto-converted)
- **Python tuples:** `(100.0, "USD")`
- **WASM/TypeScript arrays:** `[100.0, "USD"]`

### Module Imports

**Python:**
```python
from finstack.core.currency import Currency
from finstack.valuations.instruments import Bond
```

**TypeScript:**
```typescript
import { Currency, Bond } from 'finstack-wasm';
```

All WASM exports are flat at the package root.

### Error Handling

**Python:** Uses typed exceptions (`ValueError`, `RuntimeError`, `KeyError`)  
**WASM/TypeScript:** All errors are JavaScript `Error` objects

---

For detailed migration guidance, see the [Migration Guide](migration-guide.md).  
For naming conventions, see [NAMING_CONVENTIONS.md](../../../NAMING_CONVENTIONS.md).


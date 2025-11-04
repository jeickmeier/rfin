# Naming Conventions: Python ↔ WASM/TypeScript

This document provides the authoritative mapping of naming conventions between the Python (`finstack-py`) and WASM/TypeScript (`finstack-wasm`) bindings.

## Core Principle

**Both bindings follow language-idiomatic conventions:**
- **Python**: `snake_case` for functions/methods, `PascalCase` for classes
- **WASM/TypeScript**: `camelCase` for functions/methods, `PascalCase` for classes

## Quick Reference

| Pattern | Python Example | WASM/TypeScript Example |
|---------|----------------|-------------------------|
| **Standalone Functions** | `build_periods()` | `buildPeriods()` |
| **Constructor Classmethods** | `Currency.from_code()` | `Currency.fromCode()` |
| **Instance Methods** | `bond.clean_price()` | `bond.cleanPrice()` |
| **Property Getters** | `currency.code` | `currency.code` (getter) |
| **Boolean Properties** | `period.is_actual` | `period.isActual` |
| **Classes** | `Currency` | `Currency` |
| **Enums** | `DayCount.ACT_360` | `DayCount.act360()` |

## Function Naming Patterns

### Date & Calendar Utilities

| Python | WASM/TypeScript | Description |
|--------|-----------------|-------------|
| `build_periods(...)` | `buildPeriods(...)` | Build period plan |
| `build_fiscal_periods(...)` | `buildFiscalPeriods(...)` | Build fiscal periods |
| `add_months(date, n)` | `addMonths(date, n)` | Add months to date |
| `last_day_of_month(date)` | `lastDayOfMonth(date)` | Get last day of month |
| `days_in_month(year, month)` | `daysInMonth(year, month)` | Days in a month |
| `is_leap_year(year)` | `isLeapYear(year)` | Check if leap year |
| `next_imm(date)` | `nextImm(date)` | Next IMM date |
| `next_cds_date(date)` | `nextCdsDate(date)` | Next CDS date |
| `next_imm_option_expiry(...)` | `nextImmOptionExpiry(...)` | Next IMM option expiry |
| `next_equity_option_expiry(...)` | `nextEquityOptionExpiry(...)` | Next equity option expiry |
| `third_friday(year, month)` | `thirdFriday(year, month)` | Third Friday of month |
| `third_wednesday(year, month)` | `thirdWednesday(year, month)` | Third Wednesday of month |
| `imm_option_expiry(...)` | `immOptionExpiry(...)` | IMM option expiry |
| `date_to_days_since_epoch(date)` | `dateToDaysSinceEpoch(date)` | Convert date to days |
| `days_since_epoch_to_date(days)` | `daysSinceEpochToDate(days)` | Convert days to date |
| `available_calendars()` | `availableCalendars()` | List calendars |
| `available_calendar_codes()` | `availableCalendarCodes()` | List calendar codes |
| `get_calendar(code)` | `getCalendar(code)` | Get calendar by code |

### Calibration Utilities

| Python | WASM/TypeScript | Description |
|--------|-----------------|-------------|
| `validate_discount_curve(...)` | `validateDiscountCurve(...)` | Validate discount curve |
| `validate_forward_curve(...)` | `validateForwardCurve(...)` | Validate forward curve |
| `validate_hazard_curve(...)` | `validateHazardCurve(...)` | Validate hazard curve |
| `validate_inflation_curve(...)` | `validateInflationCurve(...)` | Validate inflation curve |
| `validate_vol_surface(...)` | `validateVolSurface(...)` | Validate vol surface |
| `validate_market_context(...)` | `validateMarketContext(...)` | Validate market context |

### Pricing & Risk

| Python | WASM/TypeScript | Description |
|--------|-----------------|-------------|
| `calculate_npv(...)` | `calculateNpv(...)` | Calculate NPV |
| `xirr(...)` | `xirr(...)` | XIRR calculation |
| `irr_periodic(...)` | `irrPeriodic(...)` | Periodic IRR |
| `krd_dv01_ladder(...)` | `krdDv01Ladder(...)` | Key rate DV01 ladder |
| `cs01_ladder(...)` | `cs01Ladder(...)` | Credit spread ladder |
| `results_to_json(...)` | `resultsToJson(...)` | Convert results to JSON |
| `results_to_rows(...)` | `resultsToRows(...)` | Convert results to rows |

### Portfolio Functions

| Python | WASM/TypeScript | Description |
|--------|-----------------|-------------|
| `value_portfolio(...)` | `valuePortfolio(...)` | Value entire portfolio |
| `aggregate_by_attribute(...)` | `aggregateByAttribute(...)` | Aggregate by attribute |
| `aggregate_metrics(...)` | `aggregateMetrics(...)` | Aggregate metrics |
| `group_by_attribute(...)` | `groupByAttribute(...)` | Group by attribute |
| `create_position_from_bond(...)` | `createPositionFromBond(...)` | Create position from bond |
| `create_position_from_deposit(...)` | `createPositionFromDeposit(...)` | Create position from deposit |
| `apply_scenario(...)` | `applyScenario(...)` | Apply scenario |
| `apply_and_revalue(...)` | `applyAndRevalue(...)` | Apply and revalue |

### Math & Integration

| Python | WASM/TypeScript | Description |
|--------|-----------------|-------------|
| `adaptive_quadrature(...)` | `adaptiveQuadrature(...)` | Adaptive quadrature integration |
| `adaptive_simpson(...)` | `adaptiveSimpson(...)` | Adaptive Simpson integration |
| `trapezoidal_rule(...)` | `trapezoidalRule(...)` | Trapezoidal rule integration |
| `simpson_rule(...)` | `simpsonRule(...)` | Simpson's rule integration |
| `gauss_legendre_integrate(...)` | `gaussLegendreIntegrate(...)` | Gauss-Legendre integration |
| `gauss_legendre_integrate_adaptive(...)` | `gaussLegendreIntegrateAdaptive(...)` | Adaptive Gauss-Legendre |
| `gauss_legendre_integrate_composite(...)` | `gaussLegendreIntegrateComposite(...)` | Composite Gauss-Legendre |
| `binomial_probability(...)` | `binomialProbability(...)` | Binomial probability |
| `log_binomial_coefficient(...)` | `logBinomialCoefficient(...)` | Log binomial coefficient |
| `log_factorial(...)` | `logFactorial(...)` | Log factorial |

## Class Naming

Classes use `PascalCase` in both Python and WASM:

| Python | WASM/TypeScript |
|--------|-----------------|
| `Currency` | `Currency` |
| `Money` | `Money` |
| `Bond` | `Bond` |
| `InterestRateSwap` | `InterestRateSwap` |
| `CreditDefaultSwap` | `CreditDefaultSwap` |
| `DiscountCurve` | `DiscountCurve` |
| `MarketContext` | `MarketContext` |
| `Portfolio` | `Portfolio` |

## Constructor Methods

| Python Pattern | WASM/TypeScript Pattern | Example |
|----------------|-------------------------|---------|
| `ClassName(...)` | `new ClassName(...)` | Direct constructor |
| `ClassName.from_code(...)` | `ClassName.fromCode(...)` | From code |
| `ClassName.from_json(...)` | `ClassName.fromJson(...)` | From JSON |
| `ClassName.from_tuple(...)` | `ClassName.fromTuple(...)` | From tuple/array |
| `ClassName.builder()` | `ClassName.builder()` | Builder pattern |
| `Bond.treasury(...)` | `Bond.treasury(...)` | Named constructor |
| `Bond.corporate(...)` | `Bond.corporate(...)` | Named constructor |

## Instance Method Patterns

| Python Pattern | WASM/TypeScript Pattern | Example |
|----------------|-------------------------|---------|
| `obj.method_name()` | `obj.methodName()` | Regular method |
| `obj.get_value()` | `obj.getValue()` | Getter method |
| `obj.set_value(x)` | `obj.setValue(x)` | Setter method |
| `obj.is_valid()` | `obj.isValid()` | Boolean check |
| `obj.has_property()` | `obj.hasProperty()` | Boolean check |
| `obj.to_json()` | `obj.toJson()` | Serialization |
| `obj.to_dict()` | `obj.toJson()` | To dictionary/object |
| `obj.__str__()` | `obj.toString()` | String representation |

## Property Access

### Python (Properties)
```python
currency = Currency("USD")
code = currency.code  # Property access
num_code = currency.numeric_code
```

### WASM/TypeScript (Getters)
```typescript
const currency = new Currency("USD");
const code = currency.code;  // Getter
const numCode = currency.numericCode;
```

| Python Property | WASM/TypeScript Getter | Type |
|----------------|------------------------|------|
| `currency.code` | `currency.code` | string |
| `currency.numeric_code` | `currency.numericCode` | number |
| `money.amount` | `money.amount` | number |
| `money.currency` | `currency` | Currency |
| `period.id` | `period.id` | PeriodId |
| `period.is_actual` | `period.isActual` | boolean |
| `bond.maturity` | `bond.maturity` | Date |
| `discount_curve.day_count` | `discountCurve.dayCount` | DayCount |

## Enum Patterns

### Python (Class with Constants)
```python
from finstack.core.dates import DayCount

dc = DayCount.ACT_360  # Access as class constant
yf = dc.year_fraction(start, end, ctx)
```

### WASM/TypeScript (Static Factory Methods)
```typescript
import { DayCount } from 'finstack-wasm';

const dc = DayCount.act360();  // Static factory method
const yf = dc.yearFraction(start, end, ctx);
```

| Python Enum | WASM/TypeScript Factory | Description |
|-------------|-------------------------|-------------|
| `DayCount.ACT_360` | `DayCount.act360()` | Act/360 day count |
| `DayCount.ACT_365F` | `DayCount.act365f()` | Act/365F day count |
| `DayCount.THIRTY_360` | `DayCount.thirty360()` | 30/360 day count |
| `DayCount.ACT_ACT_ISDA` | `DayCount.actActIsda()` | Act/Act ISDA |
| `Frequency.ANNUAL` | `Frequency.annual()` | Annual frequency |
| `Frequency.SEMI_ANNUAL` | `Frequency.semiAnnual()` | Semi-annual frequency |
| `Frequency.QUARTERLY` | `Frequency.quarterly()` | Quarterly frequency |
| `Frequency.MONTHLY` | `Frequency.monthly()` | Monthly frequency |
| `BusinessDayConvention.FOLLOWING` | `BusinessDayConvention.Following` | Following convention |
| `BusinessDayConvention.MODIFIED_FOLLOWING` | `BusinessDayConvention.ModifiedFollowing` | Modified following |
| `BusinessDayConvention.PRECEDING` | `BusinessDayConvention.Preceding` | Preceding convention |

## Module Structure Differences

### Python (Nested Modules)
```python
from finstack.core.currency import Currency
from finstack.core.money import Money
from finstack.core.dates import build_periods, DayCount
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.valuations.instruments import Bond, InterestRateSwap
from finstack.valuations.calibration import DiscountCurveCalibrator
from finstack.statements import ModelBuilder, Evaluator
from finstack.scenarios import ScenarioEngine, ScenarioSpec
from finstack.portfolio import Portfolio, PortfolioBuilder
```

### WASM/TypeScript (Flat Exports)
```typescript
import {
  Currency,
  Money,
  buildPeriods,
  DayCount,
  DiscountCurve,
  MarketContext,
  Bond,
  InterestRateSwap,
  DiscountCurveCalibrator,
  ModelBuilder,
  Evaluator,
  ScenarioEngine,
  ScenarioSpec,
  Portfolio,
  PortfolioBuilder
} from 'finstack-wasm';
```

**Key Difference:** Python uses nested module structure (`finstack.core.dates`) while WASM exports everything flat from the root package.

## Special Cases & Gotchas

### 1. Date Objects

**Python:**
```python
from datetime import date
pricing_date = date(2024, 1, 15)
```

**WASM/TypeScript:**
```typescript
// Use FsDate class for finstack dates
const pricingDate = new FsDate(2024, 1, 15);

// Or JavaScript Date (automatically converted)
const jsDate = new Date('2024-01-15');
```

### 2. Money Construction

**Python:**
```python
from finstack import Money, Currency

usd = Currency("USD")
amount = Money(1_000_000, usd)
# Or shorthand
amount = Money.from_code(1_000_000, "USD")
```

**WASM/TypeScript:**
```typescript
import { Money, Currency } from 'finstack-wasm';

const usd = new Currency("USD");
const amount = new Money(1_000_000, usd);
// Or shorthand
const amount = Money.fromCode(1_000_000, "USD");
```

### 3. Builder Patterns

**Python:**
```python
bond = (Bond.builder("BOND-1")
    .notional(1_000_000)
    .currency("USD")
    .coupon_rate(0.05)
    .maturity(date(2030, 1, 15))
    .build())
```

**WASM/TypeScript:**
```typescript
const bond = Bond.builder("BOND-1")
    .notional(1_000_000)
    .currency("USD")
    .couponRate(0.05)  // camelCase!
    .maturity(new FsDate(2030, 1, 15))
    .build();
```

**Note:** Builder method names also follow the language convention (snake_case vs camelCase).

### 4. Error Handling

**Python:**
```python
try:
    curve = calibrator.calibrate(quotes)
except ValueError as e:
    print(f"Validation error: {e}")
except RuntimeError as e:
    print(f"Calibration failed: {e}")
```

**WASM/TypeScript:**
```typescript
try {
    const curve = calibrator.calibrate(quotes);
} catch (e) {
    console.error("Calibration failed:", e);
    // All errors are JavaScript Error objects
}
```

## Conversion Cheatsheet

### Common Transformations

| Transform | Python → WASM/TS | WASM/TS → Python |
|-----------|------------------|------------------|
| Function/Method | `snake_case` → `camelCase` | `camelCase` → `snake_case` |
| Class | No change (PascalCase) | No change (PascalCase) |
| Property | `snake_case` → `camelCase` | `camelCase` → `snake_case` |
| Enum Value | `UPPER_CASE` → `camelCase()` | `camelCase()` → `UPPER_CASE` |
| Module Path | `finstack.core.dates` → `'finstack-wasm'` | `'finstack-wasm'` → `finstack.core.dates` |

### Quick Conversion Examples

| Python Code | WASM/TypeScript Code |
|-------------|----------------------|
| `build_periods("2024Q1..Q4", "2024Q2")` | `buildPeriods("2024Q1..Q4", "2024Q2")` |
| `currency.numeric_code` | `currency.numericCode` |
| `DayCount.ACT_360` | `DayCount.act360()` |
| `bond.clean_price()` | `bond.cleanPrice()` |
| `period.is_actual` | `period.isActual` |
| `calibrator.with_solver_kind(...)` | `calibrator.withSolverKind(...)` |
| `market.insert_discount(curve)` | `market.insertDiscount(curve)` |
| `schedule.to_array()` | `schedule.toArray()` |

## Best Practices

### 1. Consistent Naming in Your Code

When writing code that might be ported between languages:

- **Document the equivalent names** in comments
- **Use descriptive variable names** that don't rely on language conventions
- **Avoid abbreviations** that might be language-specific

### 2. Translation Scripts

Consider creating simple sed/awk scripts for mechanical conversions:

```bash
# Python to TypeScript naming
sed 's/\.([a-z_]+)(/.\1(/' | # Keep method calls
sed 's/_([a-z])/\U\1/g'        # snake_case to camelCase
```

### 3. Type Hints & TypeScript

Use type hints in Python and TypeScript definitions to catch naming errors:

```python
# Python with type hints
def process_curve(discount_curve: DiscountCurve) -> float:
    return discount_curve.zero_rate(1.0)
```

```typescript
// TypeScript with types
function processCurve(discountCurve: DiscountCurve): number {
    return discountCurve.zeroRate(1.0);
}
```

## Summary Table

| Category | Python Convention | WASM/TypeScript Convention |
|----------|------------------|----------------------------|
| Functions | `snake_case` | `camelCase` |
| Methods | `snake_case` | `camelCase` |
| Classes | `PascalCase` | `PascalCase` |
| Properties | `snake_case` | `camelCase` |
| Constants | `UPPER_CASE` | `camelCase()` or `UPPER_CASE` |
| Module Imports | `finstack.module.submodule` | `'finstack-wasm'` (flat) |
| Booleans | `is_value`, `has_property` | `isValue`, `hasProperty` |

---

**Last Updated:** Auto-generated from parity audit  
**Maintained By:** Finstack bindings team

For API-specific mappings, see [API Reference Guide](book/src/bindings/api-reference.md).


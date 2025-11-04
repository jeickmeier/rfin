# Language Bindings Documentation

Finstack provides first-class bindings for both **Python** and **TypeScript/WASM** with 100% feature parity, enabling seamless transitions between languages.

## Overview

Both bindings wrap the same Rust core library, ensuring:
- ✅ **Identical computation** - Same Rust engine, deterministic results
- ✅ **Feature parity** - 95%+ API overlap, 100% calibration parity
- ✅ **Language-idiomatic APIs** - snake_case (Python) vs camelCase (TypeScript)
- ✅ **Comprehensive docs** - Migration guides, API references, examples

## Quick Navigation

### Getting Started

- 🚀 **[Migration Guide](migration-guide.md)** - Start here to migrate code between languages
- 📖 **[API Reference](api-reference.md)** - Complete API comparison table
- 💡 **[Examples](examples.md)** - Side-by-side code examples
- 📝 **[Naming Conventions](../../../NAMING_CONVENTIONS.md)** - Quick function/method lookups

### Reference

- 📊 **[Parity Audit](../../../PARITY_AUDIT.md)** - Auto-generated feature matrix
- 🔍 **[Examples Index](../../../EXAMPLES_INDEX.md)** - Catalog of all examples
- 🏆 **[Parity Status](../../../PARITY_FINAL_STATUS.md)** - Implementation summary

## Parity Status

### Calibration APIs: 100% ✅

All 13 calibrators available in both languages:
- DiscountCurveCalibrator
- ForwardCurveCalibrator  
- HazardCurveCalibrator
- InflationCurveCalibrator
- VolSurfaceCalibrator
- BaseCorrelationCalibrator ⭐
- SimpleCalibration
- CalibrationConfig
- CalibrationReport
- RatesQuote, CreditQuote, VolQuote, InflationQuote

### Instruments: 94% ✅

35+ instruments in both bindings including:
- Fixed Income: Bonds, IRS, Swaptions, FRAs
- Credit: CDS, CDS Index, Tranches, Options
- Equity: Options, TRS, Variance Swaps
- FX: Spots, Options, Swaps, Barriers
- Exotic: Asian, Barrier, Lookback, Quanto, Autocallable
- Structured: CLO, ABS, CMBS, RMBS, Private Markets

### Statements, Scenarios, Portfolio: 90%+ ✅

- Statement modeling with forecasting
- Scenario analysis and stress testing
- Portfolio management and aggregation

## Language Differences

### Naming Conventions

| Python | TypeScript | Rule |
|--------|------------|------|
| `build_periods()` | `buildPeriods()` | snake_case → camelCase |
| `DayCount.ACT_360` | `DayCount.act360()` | CONSTANT → method() |
| `Currency("USD")` | `new Currency("USD")` | Constructor |

### Module Structure

**Python (Nested):**
```python
from finstack.core.currency import Currency
from finstack.valuations.instruments import Bond
from finstack.scenarios import ScenarioEngine
```

**TypeScript (Flat):**
```typescript
import { Currency, Bond, ScenarioEngine } from 'finstack-wasm';
```

### Key API Differences

| Feature | Python | TypeScript |
|---------|--------|------------|
| **Date Type** | `datetime.date` | `FsDate` or `Date` |
| **Tuple/Array** | `(a, b)` or `[a, b]` | `[a, b]` |
| **Dict/Object** | `{"key": "value"}` | `{"key": "value"}` (same) |
| **Operators** | `m1 + m2` works | Use `m1.add(m2)` |
| **Error Types** | `ValueError`, `RuntimeError` | Generic `Error` |

## Example Workflows

### 1. Bond Pricing

<table>
<tr><th>Python</th><th>TypeScript</th></tr>
<tr>
<td>

```python
bond = Bond.treasury(
    "US-10Y",
    1_000_000,
    "USD",
    0.0375,
    maturity,
    issue
)
```

</td>
<td>

```typescript
const bond = Bond.treasury(
    "US-10Y",
    1_000_000,
    "USD",
    0.0375,
    maturity,
    issue
);
```

</td>
</tr>
</table>

### 2. Curve Calibration

<table>
<tr><th>Python</th><th>TypeScript</th></tr>
<tr>
<td>

```python
cal = DiscountCurveCalibrator(
    "USD-OIS", date, "USD"
)
curve, report = cal.calibrate(
    quotes, market
)
```

</td>
<td>

```typescript
const cal = new DiscountCurveCalibrator(
    "USD-OIS", date, "USD"
);
const [curve, report] = cal.calibrate(
    quotes, market
);
```

</td>
</tr>
</table>

### 3. Statement Modeling

<table>
<tr><th>Python</th><th>TypeScript</th></tr>
<tr>
<td>

```python
builder = ModelBuilder.new("P&L")
builder.periods("2024Q1..Q4", "Q2")
builder.compute("ebitda", 
    "revenue - opex")
model = builder.build()
```

</td>
<td>

```typescript
const builder = ModelBuilder.new("P&L");
builder.periods("2024Q1..Q4", "Q2");
builder.compute("ebitda",
    "revenue - opex");
const model = builder.build();
```

</td>
</tr>
</table>

**Note:** The formula syntax is identical! Only method names change.

## How to Use This Documentation

### I want to migrate Python code to TypeScript

1. Read the [Migration Guide](migration-guide.md) - Section: "Python → TypeScript"
2. Use [Naming Conventions](../../../NAMING_CONVENTIONS.md) for mechanical conversion
3. Check [API Reference](api-reference.md) for specific APIs
4. Review [Examples](examples.md) for workflow patterns

### I want to migrate TypeScript code to Python

1. Read the [Migration Guide](migration-guide.md) - Section: "TypeScript → Python"
2. Reverse the [Naming Conventions](../../../NAMING_CONVENTIONS.md) patterns
3. Check [API Reference](api-reference.md) for specific APIs
4. Review [Examples](examples.md) for workflow patterns

### I want to understand what's available

1. Check [Parity Audit](../../../PARITY_AUDIT.md) for complete API inventory
2. Browse [Examples Index](../../../EXAMPLES_INDEX.md) for runnable code
3. See Python README: `finstack-py/README.md`
4. See WASM README: `finstack-wasm/README.md`

### I want to verify parity

1. Run extraction scripts: `python3 scripts/audit_*.py`
2. Generate report: `python3 scripts/compare_apis.py`
3. Review: `PARITY_AUDIT.md`
4. Check CI: `.github/workflows/bindings-parity.yml`

## Common Use Cases

### Prototype in Python, Deploy to Web

```python
# 1. Prototype in Jupyter notebook
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import DiscountCurveCalibrator

bond = Bond.treasury(...)
calibrator = DiscountCurveCalibrator(...)
curve, report = calibrator.calibrate(quotes, market)
result = pricer.price(bond, market)
```

```typescript
// 2. Port to TypeScript (mechanical name changes)
import { Bond, DiscountCurveCalibrator } from 'finstack-wasm';

const bond = Bond.treasury(...);  // Same parameters!
const calibrator = new DiscountCurveCalibrator(...);
const [curve, report] = calibrator.calibrate(quotes, market);
const result = pricer.price(bond, market);
```

**Translation time:** Minutes, not hours!

### Cross-Platform Analytics

- **Backend:** Python for data processing and batch analytics
- **Frontend:** TypeScript for interactive web dashboards
- **Mobile:** WASM for client-side calculations
- **All platforms:** Identical results (same Rust core)

## Additional Resources

### Binding-Specific Documentation

- **Python:** `finstack-py/README.md` - Installation, examples, type stubs
- **WASM:** `finstack-wasm/README.md` - Building, bundling, optimization

### API Documentation

- **Python:** Auto-generated `.pyi` stubs in `finstack-py/finstack/`
- **TypeScript:** Auto-generated `.d.ts` in `finstack-wasm/pkg/`

### Examples

- **Python:** `finstack-py/examples/scripts/` - 27 runnable scripts
- **TypeScript:** `finstack-wasm/examples/src/` - 12 interactive demos

## Maintenance

### Keeping Parity

The parity infrastructure is now in place:

1. **Before adding features:** Implement in both bindings
2. **After changes:** Run `python3 scripts/compare_apis.py`
3. **In PRs:** CI automatically checks parity
4. **Over time:** CI prevents regressions

### Updating Documentation

- **Automatic:** `PARITY_AUDIT.md` regenerated by scripts
- **Manual:** Migration guide and examples (as needed)
- **Required:** Update when adding major new features

---

**Status:** Production Ready ✅  
**Last Updated:** November 3, 2024  
**Maintained By:** Finstack bindings team


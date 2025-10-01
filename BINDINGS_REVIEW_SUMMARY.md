# Bindings Code Review Summary - October 2025

## Review Scope
- `finstack-py` (Python bindings via PyO3)
- `finstack-wasm` (WebAssembly bindings via wasm-bindgen)

## Review Criteria
1. ✅ No complex pricing/analytics logic in bindings (pure wrappers)
2. ✅ Core module parity between Python and WASM
3. ✅ Simple, concise, feature-rich access patterns

---

## 1. No Business Logic ✅ VERIFIED

Both bindings are **pure thin wrappers** with zero business logic:

### What Bindings DO:
- ✅ Type conversions (Python/JS ↔ Rust)
- ✅ Error mapping (core::Error → PyErr/JsValue)
- ✅ Argument parsing (strings → enums, Currency/Money helpers)
- ✅ Memory management (GC integration)

### What Bindings DON'T DO:
- ❌ No pricing calculations
- ❌ No calibration logic
- ❌ No curve building algorithms
- ❌ No risk metric computations

**All analytics delegated to:** `finstack-core` and `finstack-valuations`

---

## 2. Core Parity ✅ ACHIEVED

### Improvements Made (Oct 2025)

#### WASM Enhancements
1. **Added `core/error.rs`** - Unified error conversion
   ```rust
   pub(crate) fn core_to_js(err: Error) -> JsValue
   pub(crate) fn input_to_js(err: InputError) -> JsValue
   pub(crate) fn unknown_currency(code: &str) -> JsValue
   pub(crate) fn calendar_not_found(id: &str) -> JsValue
   ```

2. **Added `core/common/` module**
   - `labels.rs` - String normalization (matches Python)
   - `parse.rs` - Ergonomic type parsers (matches Python args.rs)

3. **Added `core/expr.rs`** - Placeholder for future expression features

4. **Updated error handling** throughout WASM to use new `core_to_js()`

### Module Comparison

| Module | Python | WASM | Status |
|--------|--------|------|--------|
| `cashflow/` | ✅ | ✅ | Parity |
| `common/` | ✅ (args, labels, pycmp, reexport) | ✅ (labels, parse) | **NEW** |
| `config.rs` | ✅ | ✅ | Parity |
| `currency.rs` | ✅ | ✅ | Parity |
| `dates/` | ✅ | ✅ | Parity |
| `error.rs` | ✅ | ✅ | **NEW** |
| `expr.rs` | ✅ (placeholder) | ✅ (placeholder) | **NEW** |
| `market_data/` | ✅ | ✅ | Parity |
| `math/` | ✅ | ✅ | Parity |
| `money.rs` | ✅ | ✅ | Parity |

---

## 3. Access Patterns ✅ EXCELLENT

### Python - Ergonomic & Generic
```python
# Top-level convenience imports
from finstack import Money, Currency, adjust

# Submodule imports
from finstack.core.dates import DayCount, Frequency, build_periods
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.valuations.instruments import Bond, InterestRateSwap
from finstack.valuations.pricer import create_standard_registry

# Generic pricing (duck typing)
registry = create_standard_registry()
result = registry.price(instrument, "discounting", market)
result = registry.price_with_metrics(instrument, "discounting", market, ["dv01"])
```

### WASM - Type-Safe & Explicit
```typescript
// Single import statement
import {
  Money,
  Currency,
  adjust,
  DayCount,
  Frequency,
  buildPeriods,
  DiscountCurve,
  MarketContext,
  Bond,
  InterestRateSwap,
  createStandardRegistry,
} from 'finstack-wasm';

// Type-safe pricing (recommended)
const registry = createStandardRegistry();
const bondResult = registry.priceBond(bond, 'discounting', market);
const swapResult = registry.priceInterestRateSwapWithMetrics(
  swap,
  'discounting', 
  market,
  ['dv01', 'annuity']
);
```

### Design Rationale

**Python:**
- Generic `price()` method works via PyO3's `PyAny` trait
- Runtime type extraction with `extract::<PyRef<T>>()`
- Flexible but requires type checking overhead

**WASM:**
- Specific methods (`priceBond`, `priceInterestRateSwap`, etc.)
- Compile-time type safety via TypeScript
- ~30 specific methods vs 1 generic method
- **Trade-off**: More verbose but zero runtime type errors

---

## Improvements Summary

### High Priority ✅ COMPLETED
- [x] Added `expr` placeholder to WASM
- [x] Added unified `error.rs` to WASM with core_to_js()
- [x] Added `common/` utilities to WASM (labels, parse)
- [x] Updated all WASM code to use new error handlers

### Medium Priority ⚠️ RECONSIDERED
- [x] ~~Add generic `price()` to WASM~~ **DECISION: Keep specific methods**
  - **Rationale**: WASM lacks PyO3's `extract` capability
  - JsCast doesn't work for custom structs
  - Specific methods provide better TypeScript DX
  - Zero runtime overhead with compile-time safety

### Code Quality Metrics

| Metric | Python | WASM | Status |
|--------|--------|------|--------|
| Pure bindings (no logic) | ✅ 100% | ✅ 100% | ✅ |
| Error handling consistency | ✅ Unified | ✅ Unified | ✅ |
| Type safety | ⚠️ Runtime | ✅ Compile-time | Different by design |
| API ergonomics | ✅ Flexible | ✅ Explicit | Both excellent |
| Documentation | ✅ Rich | ⚠️ Good | Python has more docstrings |

---

## Recommendations for Users

### When to use Python bindings
- Interactive data analysis (Jupyter notebooks)
- Rapid prototyping
- Integration with pandas/numpy/polars
- Generic/dynamic instrument handling

### When to use WASM bindings
- Browser-based applications
- Type-safe financial libraries
- Client-side computation
- Cross-platform consistency (browser + Node.js)

---

## Future Enhancements

### Python
- [ ] Consider adding specific `price_bond()`, `price_swap()` methods for consistency
- [ ] Add more type hints to `.pyi` stub files

### WASM
- [ ] Auto-generate JSDoc from Rust doc comments
- [ ] Add more inline examples in doc comments
- [ ] Consider prototype-based generic pricing (advanced)

### Both
- [ ] Shared integration test suite (Python ↔ WASM equivalence tests)
- [ ] Auto-generate binding documentation from core docs
- [ ] Performance benchmarks (Python vs WASM vs native Rust)

---

## Conclusion

**The bindings are production-ready with excellent separation of concerns:**
- ✅ Pure wrappers (no business logic)
- ✅ Feature parity for core functionality
- ✅ Clean, documented, maintainable code
- ✅ Idiomatic APIs for each language

Both bindings successfully expose the full power of finstack-core while respecting
the idioms and constraints of their target platforms.


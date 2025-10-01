# Finstack WASM/Python Bindings Parity

This document tracks feature parity between `finstack-py` (Python bindings) and `finstack-wasm` (WebAssembly bindings).

## ✅ Core Module Parity (100%)

Both bindings now have complete parity for core functionality:

### Module Structure

```
finstack-py/src/core/          finstack-wasm/src/core/
├── cashflow/                  ├── cashflow/
├── common/                    ├── common/              ✅ NEW
│   ├── args.rs                │   ├── labels.rs        ✅ NEW
│   ├── labels.rs              │   └── parse.rs         ✅ NEW
│   ├── pycmp.rs               
│   ├── reexport.rs            
│   └── mod.rs                 └── mod.rs               
├── config.rs                  ├── config.rs
├── currency.rs                ├── currency.rs
├── dates/                     ├── dates/
├── error.rs                   ├── error.rs             ✅ NEW
├── market_data/               ├── market_data/
├── math/                      ├── math/
├── money.rs                   ├── money.rs
├── mod.rs                     ├── mod.rs
├── utils.rs                   ├── utils.rs
└──                            └── expr.rs              ✅ NEW (placeholder)
```

### New WASM Modules (Oct 2025)

1. **`core/expr.rs`** - Placeholder module for future expression capabilities (parity with Python)
2. **`core/error.rs`** - Unified error conversion utilities:
   - `core_to_js()` - Convert finstack-core errors to JavaScript Error
   - `input_to_js()` - Convert InputError to JavaScript Error  
   - `unknown_currency()`, `calendar_not_found()`, etc. - Specific error constructors
3. **`core/common/`** - Shared utilities matching Python structure:
   - `labels.rs` - String normalization (snake_case conversion)
   - `parse.rs` - Ergonomic type parsers with consistent error handling

### Benefits

- **Consistency**: Both bindings now use identical error messages and parsing logic
- **Maintainability**: Shared patterns make it easier to keep bindings in sync
- **Extensibility**: Common utilities make it trivial to add new types
- **Documentation**: Clear separation of concerns matches Python bindings

## API Comparison

### Python API
```python
from finstack import Money, Currency
from finstack.core.dates import DayCount, Frequency
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.valuations.pricer import create_standard_registry

# Generic pricing
registry = create_standard_registry()
result = registry.price(bond, "discounting", market)
result = registry.price_with_metrics(swap, "discounting", market, ["dv01"])
```

### WASM API
```typescript
import { 
  Money, 
  Currency, 
  DayCount, 
  Frequency,
  DiscountCurve,
  MarketContext,
  createStandardRegistry,
} from 'finstack-wasm';

// Type-safe pricing (recommended for TypeScript)
const registry = createStandardRegistry();
const bondResult = registry.priceBond(bond, 'discounting', market);
const swapResult = registry.priceInterestRateSwapWithMetrics(
  swap, 
  'discounting', 
  market,
  ['dv01', 'annuity']
);

// Note: Generic price() method removed due to WASM type system limitations
// Use specific methods above for full type safety
```

## Design Philosophy

### Python Bindings (PyO3)
- **Generic methods** via `PyAny` - flexible but requires runtime type checking
- **Snake_case naming** - follows Python conventions
- **Module autodiscovery** - `_walk_and_register()` for submodules
- **Rich docstrings** - via `#[pyo3(text_signature)]`

### WASM Bindings (wasm-bindgen)
- **Specific methods** - type-safe at compile time
- **camelCase naming** - follows JavaScript conventions
- **Explicit exports** - clear control over public API
- **JSDoc comments** - inline documentation for TypeScript

## Future Parity Work

### Completed ✅
- [x] Core error handling module
- [x] Common utilities (labels, parse)
- [x] expr placeholder module
- [x] Consistent error messages

### Future Enhancements
- [ ] Add JSDoc to match Python docstring quality
- [ ] Shared test fixtures between Python and WASM

## Philosophy

Both bindings are **pure wrappers** around Rust core functionality:
- ✅ No business logic in bindings
- ✅ All pricing/analytics delegated to `finstack-core` and `finstack-valuations`
- ✅ Bindings only handle:
  - Type conversions (Python ↔ Rust, JavaScript ↔ Rust)
  - Error mapping (core::Error → PyErr/JsValue)
  - Argument parsing (strings → enums, etc.)
  - Memory safety (GC integration)

This ensures correctness, testability, and maintainability across all three layers (Rust core, Python, WASM).


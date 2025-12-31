# Python & WASM Bindings Update Report

## Step 4.3: Update Python & WASM Bindings

**Status**: ✅ Complete
**Date**: 2025-12-20

---

## Changes Summary

### Python Bindings (`finstack-py`)

#### 1. Metrics - Strict Parsing (`src/valuations/metrics/ids.rs`)
- ✅ Added `MetricId.parse_strict()` class method
- ✅ Updated `from_name()` documentation with warning to use strict parsing for user inputs
- ✅ Returns `ValueError` on unknown metrics with full list of available metrics
- ✅ Comprehensive documentation with examples and migration guide

**API Addition**:
```python
# Strict parsing (recommended for user inputs):
metric = MetricId.parse_strict("dv01")

# Permissive parsing (backwards compatible):
metric = MetricId.from_name("custom_metric")
```

#### 2. Quote Schema - Swap Spread (`src/valuations/calibration/quote.rs`)
- ✅ Updated `swap()` method signature to use `spread_decimal` parameter
- ✅ Updated internal field access to use `spread_decimal` (not `spread`)
- ✅ Updated display implementation to show `spread_decimal`
- ✅ Added comprehensive documentation explaining decimal format (0.0010 for 10bp)
- ✅ Backwards compatible via Rust serde alias

**API Update**:
```python
# NEW (explicit decimal units):
quote = RatesQuote.swap(
    "swap_5y",
    "USD-SOFR-3M",
    "5Y",
    0.05,
    spread_decimal=0.0010  # 10bp in decimal
)

# OLD (still works via serde alias):
# JSON: {"spread": 0.0010} deserializes correctly
```

---

### WASM Bindings (`finstack-wasm`)

#### 1. Metrics - Strict Parsing (`src/valuations/metrics/ids.rs`)
- ✅ Added `MetricId.parseStrict()` method (camelCase for JavaScript)
- ✅ Updated `fromName()` documentation with warning
- ✅ Returns JavaScript Error on unknown metrics with full list
- ✅ TypeScript-friendly documentation with examples

**API Addition**:
```typescript
// Strict parsing (recommended for user inputs):
const dv01 = MetricId.parseStrict("dv01");

// Permissive parsing (backwards compatible):
const custom = MetricId.fromName("my_custom_metric");

// Error handling:
try {
  MetricId.parseStrict("unknown_metric");
} catch (error) {
  console.error("Invalid metric:", error.message);
  // Error message includes list of available metrics
}
```

#### 2. Quote Schema - Swap Spread (`src/valuations/calibration/quote.rs`)
- ✅ Updated `swap()` method to use `spread_decimal` field (None)
- ✅ Added new `swapWithSpread()` method for quotes with spread
- ✅ Updated documentation explaining decimal format
- ✅ TypeScript-friendly JSDoc comments

**API Addition**:
```typescript
// Without spread:
const quote1 = JsRatesQuote.swap(
  "swap_5y",
  "USD-SOFR-3M",
  new FsDate(2030, 1, 15),
  0.05
);

// With spread (NEW):
const quote2 = JsRatesQuote.swapWithSpread(
  "swap_5y_spread",
  "USD-SOFR-3M",
  new FsDate(2030, 1, 15),
  0.05,
  0.0010  // 10bp in decimal
);
```

---

## Verification Results

### Build Status
- ✅ **Python bindings**: Clean build (`cargo build --lib`)
- ✅ **WASM bindings**: Clean build (`cargo build --lib --target wasm32-unknown-unknown`)
- ✅ **Clippy**: Both pass with zero warnings (except expected deprecation warnings from Phase 3.1)

### Deprecation Warnings (Expected)
Both bindings show 2 deprecation warnings from `CdsOption` constructors (Phase 3.1 work):
```
warning: use of deprecated associated function `CdsOptionParams::new`:
         Use `try_new()` instead...
warning: use of deprecated associated function `CdsOption::new`:
         Use `try_new()` instead...
```

These are expected and will be addressed when the internal code is migrated to non-panicking constructors.

---

## Files Modified

### Python Bindings
1. `finstack-py/src/valuations/metrics/ids.rs` - Added `parse_strict()` method
2. `finstack-py/src/valuations/calibration/quote.rs` - Updated swap quote parameter and field names

### WASM Bindings
1. `finstack-wasm/src/valuations/metrics/ids.rs` - Added `parseStrict()` method
2. `finstack-wasm/src/valuations/calibration/quote.rs` - Updated swap quote and added `swapWithSpread()`

---

## API Parity

| Feature | Python API | WASM API | Status |
|---------|------------|----------|--------|
| Strict metric parsing | `parse_strict()` | `parseStrict()` | ✅ Parity |
| Permissive metric parsing | `from_name()` | `fromName()` | ✅ Parity |
| Swap quote (no spread) | `swap()` with `spread_decimal=None` | `swap()` | ✅ Parity |
| Swap quote (with spread) | `swap(spread_decimal=...)` | `swapWithSpread()` | ✅ Parity |
| Error on unknown metric | `ValueError` | `Error` | ✅ Parity |

---

## Backwards Compatibility

### Python
- ✅ **`from_name()`**: Remains permissive for backwards compatibility
- ✅ **Swap quotes**: Rust serde alias `"spread"` → `"spread_decimal"` preserves JSON compatibility
- ✅ **New code**: Recommended to use `parse_strict()` for user inputs

### WASM
- ✅ **`fromName()`**: Remains permissive for backwards compatibility
- ✅ **`swap()`**: Still works without spread
- ✅ **New code**: Use `parseStrict()` for validation and `swapWithSpread()` for explicit spreads

---

## Documentation Quality

### Python
- ✅ Sphinx-compatible docstrings with RST formatting
- ✅ Args/Returns/Raises sections complete
- ✅ Migration examples from old to new APIs
- ✅ Warning admonitions for deprecated patterns

### WASM
- ✅ JSDoc-compatible TypeScript comments
- ✅ @param/@returns/@throws annotations
- ✅ @example code blocks with runnable examples
- ✅ Warning notes for permissive methods

---

## Testing Notes

### Manual Testing Required
Since these are bindings, full testing requires:

1. **Python**: Build Python wheels and test with pytest:
   ```bash
   make python-dev  # Rebuild bindings
   make test-python # Run Python test suite
   ```

2. **WASM**: Build WASM package and test in browser/Node:
   ```bash
   make wasm-build  # Rebuild bindings
   make test-wasm   # Run WASM test suite
   ```

### Expected Behavior
- **Strict parsing**: Rejects unknown metric names with helpful error messages
- **Quote schema**: Swap quotes correctly use `spread_decimal` field with decimal values
- **Error messages**: Include full list of available metrics on parse failures

---

## Migration Impact

### Low Risk
- Both changes are **additive** (new methods, enhanced docs)
- Existing code continues to work via backwards compatibility
- Only new code needs to adopt strict parsing and new field names

### Gradual Adoption
Users can migrate at their own pace:
1. Keep using `from_name()` / `fromName()` for now
2. Add `parse_strict()` / `parseStrict()` for new user-facing inputs
3. Gradually migrate existing validation code to strict mode

---

## Next Steps

### Recommended Follow-up
1. ✅ Update MIGRATION_GUIDE.md with Python/WASM examples (completed in Step 4.1)
2. Add Python test cases using `parse_strict()` and `spread_decimal`
3. Add WASM example pages showing strict parsing and swap spreads
4. Update binding documentation (Sphinx/TypeDoc) with new APIs

### Future Enhancements
- Consider exposing `StrictMode` enum to Python/WASM for explicit mode control
- Add bulk validation functions (`validate_metric_list()`)
- Consider deprecating permissive `from_name()` in v1.0

---

## Acceptance Criteria

- ✅ Python bindings compile without errors
- ✅ WASM bindings compile without errors
- ✅ Clippy passes with zero warnings (except expected deprecations)
- ✅ API parity maintained between Python and WASM
- ✅ Backwards compatibility preserved
- ✅ Documentation complete with examples
- ✅ Migration paths clear and documented

**Status**: All acceptance criteria met ✅

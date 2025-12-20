# Technical Specification: Market Convention Refactors

## Executive Summary

This specification addresses critical market convention compliance issues identified in the finstack valuations crate audit. The changes span multiple severity levels (Critical, Major, Moderate) and affect core pricing reliability, FX settlement accuracy, and risk reporting correctness.

**Complexity Level**: **HARD**

This is a complex, high-risk refactor involving:
- Multiple critical safety issues (silent errors, incorrect calculations)
- Convention alignment requiring market-standard expertise
- Breaking API changes across multiple crates
- Cross-cutting changes in error handling philosophy
- Potential impact on all downstream pricing and risk systems

---

## Technical Context

### Language & Dependencies
- **Language**: Rust (stable)
- **Primary Crates**:
  - `finstack/valuations` (main changes)
  - `finstack/core` (supporting changes for dates/errors)
  - `finstack-py` (Python bindings updates)
  - `finstack-wasm` (WASM bindings updates)
- **Key Dependencies**:
  - `finstack-core` for dates, calendars, money, error types
  - `hashbrown` for HashMap (metrics registry)
  - `indexmap` for ordered maps
  - `time` crate for date arithmetic
  - `serde` for serialization

### Existing Architecture
The codebase follows a modular architecture:
```
finstack/valuations/
├── src/
│   ├── instruments/         # Product definitions
│   │   └── common/
│   │       ├── traits.rs    # Instrument trait
│   │       └── fx_dates.rs  # FX settlement logic (NEEDS FIX)
│   ├── metrics/
│   │   └── core/
│   │       ├── registry.rs  # Metric computation (NEEDS FIX)
│   │       └── ids.rs       # Metric identifiers (NEEDS FIX)
│   ├── calibration/
│   │   └── targets/
│   │       └── discount.rs  # Curve calibration (NEEDS FIX)
│   ├── market/
│   │   └── build/
│   │       └── rates.rs     # Quote builder (NEEDS FIX)
│   └── results/
│       └── dataframe.rs     # Export mappings (NEEDS FIX)
```

---

## Implementation Approach

### Phase 1: Critical Safety Fixes (Week 1)
**Goal**: Eliminate silent failures and incorrect calculations that pose the highest risk.

#### 1.1 Metrics Framework Strictness

**Problem**: Metrics computation silently returns `0.0` on errors and accepts unknown metric names.

**Files to Modify**:
- `finstack/valuations/src/metrics/core/registry.rs`
- `finstack/valuations/src/metrics/core/ids.rs`
- `finstack/core/src/error.rs` (add new error variants)

**Changes**:

1. **Add strict mode to MetricRegistry** (`registry.rs`):
   - Add `StrictMode` enum: `Strict | BestEffort`
   - Modify `compute()` to take a mode parameter
   - In `Strict` mode: return `Err` for missing metrics, failed calculations, non-applicable metrics
   - In `BestEffort` mode: preserve current behavior with explicit warnings
   - Default to `Strict` for all production paths

2. **Add strict parsing** (`ids.rs`):
   - Add `MetricId::parse_strict(s: &str) -> Result<MetricId>` 
   - Returns `Err(Error::UnknownMetric)` for unknown metric names
   - Keep `from_str()` permissive for backwards compatibility
   - Update all public APIs to use `parse_strict()` for user inputs

3. **Fix dependency resolution** (`registry.rs:L290-298`):
   - Change `let _ = self.visit_metric(...)` to propagate errors
   - Add cycle detection with readable error messages including the cycle path
   - Return `Err(Error::CircularDependency { path: Vec<MetricId> })`

**New Error Variants** (in `finstack/core/src/error.rs`):
```rust
pub enum Error {
    // ... existing variants
    
    /// Unknown metric requested
    UnknownMetric { 
        metric_id: String,
        available: Vec<String>,
    },
    
    /// Metric not applicable to instrument type
    MetricNotApplicable {
        metric_id: String,
        instrument_type: String,
    },
    
    /// Metric calculation failed
    MetricCalculationFailed {
        metric_id: String,
        cause: Box<Error>,
    },
    
    /// Circular dependency in metrics
    CircularDependency {
        path: Vec<String>,
    },
}
```

**API Impact**:
- `MetricRegistry::compute()` signature changes to include mode
- Breaking change: some existing calls will error instead of returning 0.0
- Migration: add explicit mode parameter or use new `compute_strict()` / `compute_best_effort()` convenience methods

#### 1.2 Calibration Residual Normalization

**Problem**: Discount curve global residuals divide by `1.0` instead of `residual_notional`.

**Files to Modify**:
- `finstack/valuations/src/calibration/targets/discount.rs`

**Changes**:

In `DiscountCurveTarget::calculate_residuals()` (line ~57):
```rust
// BEFORE:
residuals[i] = pv / 1.0;

// AFTER:
residuals[i] = pv / self.residual_notional;
```

**Validation**:
- Add test: same curve calibration with `residual_notional = 1.0` and `1_000_000.0` should produce identical curves
- Verify max residual ≤ `1e-8` in normalized units
- Golden test against known calibration results

**API Impact**: None (internal calculation fix)

---

### Phase 2: Market Convention Alignment (Week 2)

#### 2.1 FX Spot Date Convention

**Problem**: FX spot uses calendar days instead of joint-calendar business days.

**Files to Modify**:
- `finstack/valuations/src/instruments/common/fx_dates.rs`
- `finstack/core/src/dates/` (may need helper functions)

**Changes**:

1. **Add joint business day arithmetic** (new function):
```rust
/// Add N business days on a joint calendar (day is business if both calendars agree).
pub fn add_joint_business_days(
    start: Date,
    n_days: u32,
    bdc: BusinessDayConvention,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> Result<Date> {
    let base_cal = resolve_calendar(base_cal_id)?; // Now returns Result
    let quote_cal = resolve_calendar(quote_cal_id)?;
    
    let mut date = start;
    let mut count = 0u32;
    
    // Iterate until we've found n_days that are business days on BOTH calendars
    const MAX_ITERS: u32 = n_days * 5; // Safety limit
    let mut iters = 0;
    
    while count < n_days && iters < MAX_ITERS {
        date = date + time::Duration::days(1);
        
        // Check if business day on both calendars
        if base_cal.as_ref().is_business_day(date) 
           && quote_cal.as_ref().is_business_day(date) {
            count += 1;
        }
        
        iters += 1;
    }
    
    if iters >= MAX_ITERS {
        return Err(Error::Date { 
            message: format!("Failed to find {} joint business days from {}", n_days, start)
        });
    }
    
    Ok(date)
}
```

2. **Update `roll_spot_date()`** (lines 61-70):
```rust
pub fn roll_spot_date(
    trade_date: Date,
    spot_lag_days: u32,
    bdc: BusinessDayConvention,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> Result<Date> {
    // Use joint business day counting instead of calendar days
    add_joint_business_days(
        trade_date,
        spot_lag_days,
        bdc,
        base_cal_id,
        quote_cal_id,
    )
}
```

3. **Make calendar resolution strict** (lines 12-19):
```rust
fn resolve_calendar(cal_id: Option<&str>) -> Result<CalendarWrapper> {
    if let Some(id) = cal_id {
        if let Some(resolved) = CalendarRegistry::global().resolve_str(id) {
            return Ok(CalendarWrapper::Borrowed(resolved));
        }
        
        // Error instead of silent fallback
        return Err(Error::CalendarNotFound { 
            calendar_id: id.to_string(),
            hint: "Use CalendarRegistry::available_calendars() to see valid IDs",
        });
    }
    
    // Only use weekends_only if explicitly None (not as fallback)
    Ok(CalendarWrapper::Owned(weekends_only()))
}
```

**New Error Variant**:
```rust
pub enum Error {
    // ...
    CalendarNotFound {
        calendar_id: String,
        hint: &'static str,
    },
}
```

**Testing**:
- Test FX spot around base/quote holidays (verify correct skip behavior)
- Test unknown calendar ID → error
- Golden test against known FX settlement dates

**API Impact**:
- `resolve_calendar()` now returns `Result` (breaking internal change)
- FX settlement dates will shift for trades near holidays (correctness fix)
- Add compatibility flag if gradual migration needed: `allow_calendar_fallback: bool`

#### 2.2 Quote Units Clarification

**Problem**: Swap spread field ambiguous (bp vs decimal).

**Files to Modify**:
- `finstack/valuations/src/market/quotes/rates.rs` (RateQuote enum)
- `finstack/valuations/src/market/build/rates.rs` (builder)

**Changes**:

1. **Rename field with explicit units**:
```rust
// In RateQuote::Swap variant:
pub enum RateQuote {
    Swap {
        id: QuoteId,
        index: IndexId,
        pillar: Pillar,
        rate: f64, // Still decimal
        
        // BEFORE:
        // spread: Option<f64>, // Ambiguous!
        
        // AFTER (choose ONE approach):
        
        // Option A: Store as bp (recommended)
        spread_bp: Option<f64>,
        
        // Option B: Store as decimal with clear name
        spread_decimal: Option<f64>,
    },
    // ...
}
```

2. **Remove conversion in builder** (line 373):
```rust
// BEFORE:
if let Some(s) = spread {
    swap.float.spread_bp = *s * 10000.0;
}

// AFTER (if using Option A):
if let Some(spread_bp) = spread_bp {
    swap.float.spread_bp = *spread_bp; // Direct assignment
}

// AFTER (if using Option B):
if let Some(spread_decimal) = spread_decimal {
    swap.float.spread_bp = *spread_decimal * 10000.0; // Explicit conversion
}
```

3. **Add serde alias for backwards compatibility**:
```rust
#[serde(alias = "spread")] // Old name still works on deserialization
spread_bp: Option<f64>,
```

**Validation**:
- Add test: quote with `spread_bp = 10.0` produces swap with `float.spread_bp = 10.0`
- Document unit convention in rustdoc

**API Impact**:
- Serde breaking change (mitigated by alias)
- Programmatic API: field rename (breaking)
- Migration guide: rename `spread` → `spread_bp` in JSON configs

---

### Phase 3: API Safety & Reporting (Week 3)

#### 3.1 Remove Panicking Constructors

**Problem**: Public `new()` methods panic via `expect()`.

**Files to Modify**:
- `finstack/valuations/src/instruments/cds_option/*.rs`
- Any other instruments with panicking `new()` (search: `grep -r "\.expect.*new\(\)" src/instruments/`)

**Changes**:

1. **Remove or gate panicking constructors**:
```rust
impl CdsOption {
    // OPTION A: Remove entirely (recommended)
    // Delete `pub fn new()` method
    
    // OPTION B: Keep only for tests
    #[cfg(test)]
    pub fn new(...) -> Self {
        Self::try_new(...).expect("Invalid CdsOption parameters")
    }
    
    // Keep this as the only public constructor
    pub fn try_new(...) -> Result<Self> {
        // Validation logic
        // ...
        Ok(Self { ... })
    }
}
```

2. **Add clippy lints to prevent regression**:
```rust
// In lib.rs:
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

// For specific cases where panic is OK (tests):
#[allow(clippy::expect_used)]
```

**Testing**:
- Verify all instrument construction via `try_new()` paths
- Invalid parameters return structured errors

**API Impact**:
- Breaking: `new()` methods removed or gated
- Migration: use `try_new()` and handle `Result`

#### 3.2 Results Export Metric Mapping

**Problem**: `to_row()` uses wrong metric key strings.

**Files to Modify**:
- `finstack/valuations/src/results/dataframe.rs`

**Changes**:

```rust
// BEFORE (lines 48-54):
duration: self
    .measures
    .get("duration")
    .or_else(|| self.measures.get("modified_duration"))
    .copied(),

// AFTER:
duration: self
    .measures
    .get(MetricId::DurationMod.as_str())
    .or_else(|| self.measures.get(MetricId::DurationMac.as_str()))
    .copied(),

// Better: Use a helper function
impl ValuationResult {
    fn get_measure(&self, id: MetricId) -> Option<f64> {
        self.measures.get(id.as_str()).copied()
    }
}

// Then:
duration: self.get_measure(MetricId::DurationMod)
    .or_else(|| self.get_measure(MetricId::DurationMac)),
```

**Add unit test**:
```rust
#[test]
fn test_to_row_metric_mapping() {
    let mut measures = IndexMap::new();
    measures.insert(MetricId::DurationMod.as_str().to_string(), 5.25);
    measures.insert(MetricId::Dv01.as_str().to_string(), 1250.0);
    
    let result = ValuationResult::stamped_with_meta(...)
        .with_measures(measures);
    
    let row = result.to_row();
    
    assert_eq!(row.duration, Some(5.25));
    assert_eq!(row.dv01, Some(1250.0));
}
```

**API Impact**: None (internal fix)

---

## Source Code Structure Changes

### Files to Create
- None (all changes to existing files)

### Files to Modify (Summary)

| File | Lines Changed | Severity | Breaking |
|------|---------------|----------|----------|
| `valuations/src/metrics/core/registry.rs` | ~50 | Critical | Yes |
| `valuations/src/metrics/core/ids.rs` | ~20 | Critical | Yes |
| `core/src/error.rs` | ~30 | Critical | No |
| `valuations/src/calibration/targets/discount.rs` | 1 | Critical | No |
| `valuations/src/instruments/common/fx_dates.rs` | ~60 | Major | Yes |
| `valuations/src/market/quotes/rates.rs` | ~5 | Major | Yes |
| `valuations/src/market/build/rates.rs` | ~5 | Major | No |
| `valuations/src/instruments/cds_option/*.rs` | ~10 per file | Major | Yes |
| `valuations/src/results/dataframe.rs` | ~10 | Moderate | No |

**Total estimate**: ~300 lines changed across 10-15 files

### Files to Delete
- None

### New Dependencies
- None

---

## Data Model / API / Interface Changes

### Breaking Changes

1. **MetricRegistry API** (Critical Breaking)
   - `compute()` adds `mode: StrictMode` parameter
   - New convenience methods: `compute_strict()`, `compute_best_effort()`
   - Old behavior available via explicit `BestEffort` mode

2. **MetricId Parsing** (Critical Breaking)
   - `MetricId::parse_strict()` introduced (recommended for new code)
   - `FromStr` unchanged (backwards compatible)
   - User-facing configs should migrate to strict parsing

3. **FX Date Resolution** (Major Breaking)
   - `resolve_calendar()` returns `Result` instead of infallible
   - FX spot settlement dates change for trades near holidays (correctness)
   - May require market data updates if settlement dates shift

4. **Quote Schema** (Major Breaking)
   - `RateQuote::Swap::spread` → `spread_bp` (or `spread_decimal`)
   - Serde alias preserves JSON backwards compatibility
   - Programmatic API breaks (field rename)

5. **Instrument Constructors** (Major Breaking)
   - Panicking `new()` methods removed or gated
   - All construction via `try_new() -> Result<Self>`

### Non-Breaking Changes

1. **Calibration residual normalization** (internal fix)
2. **Results export metric keys** (internal fix)

### Migration Path

**For Library Users**:

```rust
// BEFORE:
let metrics = registry.compute(&metric_ids, &mut context)?;

// AFTER (Option 1 - Explicit mode):
let metrics = registry.compute(&metric_ids, &mut context, StrictMode::Strict)?;

// AFTER (Option 2 - Convenience method):
let metrics = registry.compute_strict(&metric_ids, &mut context)?;

// BEFORE:
let spot = roll_spot_date(trade, 2, bdc, Some("US"), Some("EU"))?;

// AFTER (may fail if calendar missing):
let spot = roll_spot_date(trade, 2, bdc, Some("US"), Some("EU"))
    .or_else(|e| {
        // Handle missing calendar explicitly
        eprintln!("Warning: {}", e);
        roll_spot_date(trade, 2, bdc, None, None) // Fallback to weekends-only
    })?;

// BEFORE:
let opt = CdsOption::new(...); // Could panic!

// AFTER:
let opt = CdsOption::try_new(...)
    .map_err(|e| format!("Invalid CDS option: {}", e))?;
```

**For Config Files** (JSON):

```json
{
  "swap_quote": {
    "id": "USD-OIS-5Y",
    "spread": 10.0  ← Old field (still works via serde alias)
  }
}
```
→
```json
{
  "swap_quote": {
    "id": "USD-OIS-5Y",
    "spread_bp": 10.0  ← New field (explicit units)
  }
}
```

---

## Verification Approach

### Unit Tests (Per Module)

1. **Metrics Framework** (`metrics/core/`):
   - `test_strict_mode_unknown_metric` - unknown metric → error
   - `test_strict_mode_calculation_failure` - failed calculator → error with cause
   - `test_strict_mode_not_applicable` - metric not for instrument type → error
   - `test_best_effort_mode_fallback` - best effort → 0.0 with warning
   - `test_circular_dependency` - cycle → error with path
   - `test_parse_strict_unknown` - strict parsing → error
   - `test_parse_strict_known` - strict parsing → success

2. **Calibration** (`calibration/targets/`):
   - `test_residual_normalization_invariance` - notional 1.0 vs 1M → same curve
   - `test_residual_max_tolerance` - max residual ≤ 1e-8 (normalized)

3. **FX Dates** (`instruments/common/`):
   - `test_joint_business_days` - count across base/quote holidays
   - `test_spot_settlement_near_holiday` - verify skip behavior
   - `test_missing_calendar_error` - unknown ID → error
   - `test_explicit_none_weekends_only` - None calendars → weekends-only

4. **Quote Units** (`market/build/`):
   - `test_swap_spread_bp_units` - spread_bp=10 → float.spread_bp=10
   - `test_serde_backwards_compat` - old JSON "spread" field → deserializes

5. **Constructor Safety** (`instruments/`):
   - `test_try_new_invalid_params` - invalid → Err
   - `test_try_new_valid_params` - valid → Ok

6. **Results Export** (`results/`):
   - `test_to_row_duration_mapping` - duration_mod present → row.duration filled
   - `test_to_row_dv01_mapping` - dv01 present → row.dv01 filled

### Integration Tests

1. **End-to-End Calibration**:
   - Build OIS curve with 50 quotes (deposits, FRAs, swaps)
   - Verify all residuals ≤ 1e-8 (normalized)
   - Compare against golden reference curve

2. **FX Settlement Workflow**:
   - Price FX forward with trade date near USD/EUR holidays
   - Verify settlement dates match ISDA conventions
   - Compare before/after this change (document expected shifts)

3. **Multi-Metric Valuation**:
   - Price bond with 10 metrics (DV01, duration, convexity, etc.)
   - Strict mode: verify all succeed or error clearly
   - Best effort mode: verify fallback behavior with warnings

### Regression Tests

1. **Golden File Tests**:
   - Capture current calibration results before changes
   - Re-run after fixes
   - Allow for *expected* differences (FX dates, residual normalization)
   - Verify no *unexpected* differences

2. **Parity Tests** (if applicable):
   - Bond pricing: match Bloomberg/QuantLib within tolerance
   - FX settlement: match vendor calendars (ISDA, Bloomberg)

### Performance Benchmarks

- Calibration: 200-quote curve (before/after residual fix)
- Metrics: 1000-instrument portfolio with 10 metrics each
- FX dates: 10,000 spot date calculations

**Acceptance Criteria**:
- No >10% performance regression in any benchmark
- Strict mode should be <5% slower than best effort

---

## Testing & Validation Matrix

| Component | Test Type | Success Criteria |
|-----------|-----------|------------------|
| Metrics strict mode | Unit | Unknown metric → `Err(UnknownMetric)` |
| Metrics strict mode | Unit | Failed calc → `Err(MetricCalculationFailed)` |
| Metrics cycle detection | Unit | Cycle → `Err(CircularDependency)` with path |
| Calibration residuals | Unit | Notional invariance test passes |
| Calibration residuals | Integration | Max residual ≤ 1e-8 (normalized) |
| FX spot date | Unit | Joint business day count correct across holidays |
| FX spot date | Integration | Settlement dates match ISDA standard |
| FX calendar missing | Unit | Unknown ID → `Err(CalendarNotFound)` |
| Quote units | Unit | spread_bp → no conversion |
| Quote serde | Unit | Old "spread" field deserializes to spread_bp |
| Constructor safety | Unit | Invalid params → `Err` not panic |
| Results export | Unit | duration_mod → row.duration |
| Clippy lints | Static | `cargo clippy -- -D warnings` passes |
| Benchmarks | Perf | <10% regression allowed |

---

## Risk Assessment & Mitigation

### High-Risk Changes

1. **Metrics strict mode default**
   - **Risk**: Breaks existing code that relies on silent 0.0 fallback
   - **Mitigation**: 
     - Provide explicit `BestEffort` mode for gradual migration
     - Document migration path clearly
     - Add deprecation warnings in 0.x release before making strict default

2. **FX settlement date shifts**
   - **Risk**: Pricing changes for instruments near holidays
   - **Mitigation**:
     - Document expected date shifts in migration guide
     - Provide comparison tool to check existing trades
     - Add flag to preserve legacy behavior temporarily

3. **Calibration residual scaling**
   - **Risk**: Existing calibrations may fail with new tolerances
   - **Mitigation**:
     - Tolerances are already per-unit; fix improves consistency
     - Re-run golden tests to establish new baselines
     - Document expected changes

### Medium-Risk Changes

1. **Quote field rename**
   - **Risk**: Serde deserialization fails for old configs
   - **Mitigation**: Use `#[serde(alias)]` for backwards compatibility

2. **Constructor removals**
   - **Risk**: Compile errors in user code
   - **Mitigation**: 
     - Deprecate in 0.x, remove in 1.0
     - Provide clear compiler errors with migration hints

### Low-Risk Changes

- Results export key mapping (internal only)

---

## Rollback Plan

### Phase-wise Rollback

Each phase is independently deployable:

**Phase 1 (Critical Safety)**:
- Metrics: Keep `BestEffort` as default in 0.x, switch to `Strict` in 1.0
- Calibration: Pure correctness fix, no rollback needed (validates via tests)

**Phase 2 (Conventions)**:
- FX dates: Add feature flag `legacy_fx_settlement` to preserve old behavior
- Quote units: Serde alias allows gradual migration

**Phase 3 (API Safety)**:
- Constructors: Deprecation warnings in 0.x, removal in 1.0
- Results export: Internal change, no rollback needed

### Emergency Rollback Procedure

If critical issues arise in production:

1. **Revert to previous release** (full rollback)
2. **Disable strict mode via feature flag**: Add Cargo feature `metrics_strict_mode` (default off)
3. **Use legacy FX flag**: `legacy_fx_settlement = true` in config

---

## Timeline & Dependencies

### Phase 1 (Week 1): Critical Safety - 5 days
- Day 1-2: Metrics framework strict mode + error types
- Day 3: Calibration residual fix + tests
- Day 4: Dependency resolution + cycle detection
- Day 5: Integration tests + documentation

**Dependencies**: None (standalone changes)

### Phase 2 (Week 2): Market Conventions - 5 days
- Day 1-2: FX joint calendar business day logic
- Day 3: Quote units clarification + serde
- Day 4: Calendar resolution strictness
- Day 5: Integration tests + golden file updates

**Dependencies**: Phase 1 error types

### Phase 3 (Week 3): API Safety & Reporting - 5 days
- Day 1-2: Remove panicking constructors across instruments
- Day 3: Clippy lints + static analysis
- Day 4: Results export fixes
- Day 5: Full regression suite + benchmarks

**Dependencies**: Phases 1-2 for full testing

### Buffer (Week 4): Documentation & Migration - 3 days
- Migration guide
- API documentation updates
- Release notes
- Deprecation warnings

**Total**: 18 working days (~4 weeks)

---

## Open Questions & Decisions Needed

1. **Metrics Strict Mode Default**:
   - **Question**: Make `Strict` the default immediately, or gradual migration via feature flag?
   - **Recommendation**: Feature flag in 0.x, default in 1.0
   - **Decision needed by**: Phase 1 start

2. **Quote Unit Convention**:
   - **Question**: Store as `spread_bp` (basis points) or `spread_decimal` (decimal)?
   - **Recommendation**: `spread_bp` (aligns with internal `float.spread_bp` field)
   - **Decision needed by**: Phase 2 start

3. **Constructor Migration Timeline**:
   - **Question**: Remove `new()` immediately or deprecate first?
   - **Recommendation**: Deprecate in next minor release, remove in 1.0
   - **Decision needed by**: Phase 3 start

4. **Legacy FX Flag Lifetime**:
   - **Question**: How long to maintain `legacy_fx_settlement` flag?
   - **Recommendation**: 2 minor releases, then remove
   - **Decision needed by**: Phase 2 start

---

## Success Criteria

### Functional
- ✅ All unit tests pass (100% coverage for changed code)
- ✅ Integration tests pass (calibration, FX settlement, multi-metric)
- ✅ Regression tests show only *expected* differences
- ✅ Clippy/fmt pass with no warnings

### Performance
- ✅ Benchmarks show <10% regression (or justified)
- ✅ Strict mode overhead <5% vs best effort

### Compliance
- ✅ FX settlement matches ISDA conventions (verified via test cases)
- ✅ Calibration tolerances work consistently across notionals
- ✅ Metric errors are actionable (include context, not silent zeros)

### Documentation
- ✅ Migration guide published
- ✅ API documentation updated
- ✅ Breaking changes documented in CHANGELOG
- ✅ Release notes drafted

---

## Post-Implementation

### Monitoring Plan
- Track metric computation error rates (strict mode)
- Monitor calibration residuals (verify no regressions)
- Log FX settlement date shifts (validate corrections)

### Future Enhancements (Out of Scope)
- Tenor bump exact matching (audit finding #9)
- Inflation curve currency inference removal (audit finding #10)
- Carry computation for multi-day attribution (audit finding #12)
- Convention registry initialization safety (audit finding #11)

### Technical Debt Cleanup
- Standardize all error handling to use new error variants
- Add more comprehensive clippy lints across codebase
- Expand golden file test coverage
- Document all market conventions in dedicated guide

---

## References

- **Audit Report**: Task description (JSON findings)
- **ISDA Conventions**: FX spot settlement (T+2 business days, joint calendar)
- **Rust API Guidelines**: Error handling best practices
- **Project Rules**: 
  - `.cursor/rules/rust/crates/core.mdc`
  - `.cursor/rules/rust/crates/valuations.mdc`
  - `.cursor/rules/project-rules.mdc`

---

## Appendix: Detailed Code Examples

### Example A: Metrics Strict Mode Usage

```rust
use finstack_valuations::metrics::{MetricRegistry, StrictMode};

// Production code (strict by default):
let metrics = registry.compute_strict(&metric_ids, &mut context)?;

// Migration path (gradual):
#[cfg(feature = "strict_metrics")]
let mode = StrictMode::Strict;
#[cfg(not(feature = "strict_metrics"))]
let mode = StrictMode::BestEffort;

let metrics = registry.compute(&metric_ids, &mut context, mode)?;

// Error handling:
match registry.compute_strict(&metric_ids, &mut context) {
    Ok(metrics) => { /* use metrics */ },
    Err(Error::UnknownMetric { metric_id, available }) => {
        eprintln!("Unknown metric: {}", metric_id);
        eprintln!("Available: {:?}", available);
    },
    Err(Error::MetricCalculationFailed { metric_id, cause }) => {
        eprintln!("Failed to compute {}: {}", metric_id, cause);
    },
    Err(e) => return Err(e),
}
```

### Example B: FX Joint Calendar

```rust
use finstack_valuations::instruments::common::fx_dates::{
    add_joint_business_days, roll_spot_date
};

// Explicit joint business day counting:
let trade_date = Date::from_calendar_date(2024, 12, 30)?; // Monday
let spot = roll_spot_date(
    trade_date,
    2, // T+2
    BusinessDayConvention::Following,
    Some("US"), // New Year's Day 2025
    Some("EU"), // New Year's Day 2025
)?;
// Result: 2025-01-03 (skips Jan 1-2 holidays)

// Error handling for missing calendar:
match roll_spot_date(trade_date, 2, bdc, Some("UNKNOWN"), None) {
    Ok(date) => { /* use date */ },
    Err(Error::CalendarNotFound { calendar_id, hint }) => {
        eprintln!("Calendar {} not found. {}", calendar_id, hint);
        // Fallback or fail explicitly
    },
    Err(e) => return Err(e),
}
```

### Example C: Quote Units

```rust
use finstack_valuations::market::quotes::rates::RateQuote;

// New explicit schema:
let quote = RateQuote::Swap {
    id: QuoteId::new("USD-OIS-5Y"),
    index: IndexId::new("USD-SOFR-OIS"),
    pillar: Pillar::Tenor("5Y".parse()?),
    rate: 0.0450,        // Decimal (4.50%)
    spread_bp: Some(10.0), // Basis points (10bp)
};

// No conversion needed in builder:
let instrument = build_rate_instrument(&quote, &ctx)?;
assert_eq!(instrument.float.spread_bp, 10.0); // Direct
```

---

**End of Technical Specification**

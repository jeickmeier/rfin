# Migration Guide: Market Convention Refactors (0.8.0)

> **Comprehensive guide for migrating to finstack 0.8.0 with market convention compliance fixes**

**Version**: 0.8.0
**Release Date**: December 2024
**Breaking Changes**: Yes (see details below)

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start: Migration Decision Tree](#quick-start-migration-decision-tree)
3. [Breaking Changes by Phase](#breaking-changes-by-phase)
   - [Phase 1: Critical Safety Fixes](#phase-1-critical-safety-fixes)
   - [Phase 2: Market Convention Alignment](#phase-2-market-convention-alignment)
   - [Phase 3: API Safety & Constructor Removal](#phase-3-api-safety--constructor-removal)
4. [Deprecations (0.4.1)](#deprecations-041)
5. [Migration Strategies](#migration-strategies)
6. [Code Examples: Before & After](#code-examples-before--after)
7. [Error Handling Updates](#error-handling-updates)
8. [Testing Recommendations](#testing-recommendations)
9. [FAQ](#faq)
10. [Support & Resources](#support--resources)

---

## Overview

Version 0.8.0 introduces critical fixes for market convention compliance and safety issues that could lead to incorrect pricing and risk calculations. The changes address:

### Why These Changes?

**Critical Issues Fixed**:

- 🔴 **Silent metric failures**: Errors were suppressed, returning `0.0` for failed/unknown metrics
- 🔴 **Incorrect FX settlement**: Used calendar days instead of joint business days
- 🔴 **Calibration scaling bug**: Residuals not normalized correctly
- 🟠 **Quote unit ambiguity**: Decimal vs basis point confusion in swap spreads
- 🟠 **Panicking constructors**: Library code could panic instead of returning errors

### Migration Complexity

| User Type | Estimated Effort | Strategy |
|-----------|-----------------|----------|
| **Application developers** | 2-4 hours | Follow decision tree, handle new errors |
| **Library authors** | 4-8 hours | Update APIs, add strict mode support |
| **Large codebases** | 1-2 days | Use gradual migration approach (Phase 1 → 2 → 3) |

---

## Quick Start: Migration Decision Tree

```
START: Do you use metrics computation (DV01, CS01, Greeks)?
├── YES → Go to Phase 1 (Required)
│   ├── Do you parse metric names from config/CLI?
│   │   ├── YES → Update to MetricId::parse_strict()
│   │   └── NO → Just handle new error types
│   └── Choose a strict-mode error handling strategy
└── NO → Skip Phase 1

Do you price FX instruments or use multi-currency portfolios?
├── YES → Go to Phase 2 (Recommended)
│   ├── Update calendar error handling
│   └── Verify FX spot dates (breaking: behavior changed)
└── NO → Skip Phase 2

Do you use CdsOption or other instruments with panicking constructors?
├── YES → Go to Phase 3 (Breaking)
│   └── Update to try_new() variants (constructors removed)
└── NO → Skip Phase 3

Do you use calibration heavily?
├── YES → Test calibration after Phase 1 residual fix
│   └── Should be identical results (just better scaling)
└── NO → Done
```

## New deprecations: valuations public API surface

- **Canonical imports** (use these going forward):
  - `finstack_valuations::instruments::{Instrument, Attributes, instrument_to_arc, build_with_metrics_dyn, Bond, InterestRateSwap, ...}`
  - `finstack_valuations::pricer::{PricerRegistry, ModelKey, InstrumentType, create_standard_registry}`
  - `finstack_valuations::metrics::{MetricId, MetricRegistry, MetricContext, standard_registry}` (plus VaR via `metrics::risk`)
  - `finstack_valuations::cashflow::{CashflowProvider, schedule_from_dated_flows, AccrualConfig, AccrualMethod, ExCouponRule, accrued_interest_amount}`
  - `finstack_valuations::results::{ValuationResult, ValuationRow, ResultsMeta, results_to_rows}`
  - `finstack_valuations::covenants::{Covenant, CovenantType, CovenantEngine, GenericCovenantForecast, CovenantForecastConfig}`
  - `finstack_valuations::attribution::{AttributionMethod, AttributionEnvelope, attribute_pnl_parallel, attribute_pnl_waterfall, attribute_pnl_metrics_based, JsonEnvelope}`
  - `finstack_valuations::calibration::{api::*, SolverConfig, CalibrationConfig, ValidationConfig}` and bump helpers `calibration::bumps::{bump_discount_curve_synthetic, bump_hazard_spreads, bump_inflation_rates, BumpRequest}`
- **Deprecated paths** (no longer supported): deep module imports such as `instruments::common::models`, `calibration::bumps::rates`/`hazard`/`inflation`, `covenants::engine`/`forward`, `attribution::types`/`spec`/`metrics_based`, `cashflow::traits`, `cashflow::accrual`, `results::dataframe`, and legacy top‑level instrument module aliases like `instruments::bond`, `instruments::swaption`, `instruments::cds_option`, `instruments::fx_forward`. Use category modules under `instruments::{fixed_income, rates, credit_derivatives, equity, fx, commodity, exotics}`.

### Calibration API slimming

The calibration module now has a single supported pathway: the v2 plan schema/engine plus the `calibration::bumps` helpers. The following items are deprecated and will be removed in the next major release:

| Deprecated | Replacement |
| --- | --- |
| `finstack_valuations::calibration::CalibrationSolveMethod` | `finstack_valuations::calibration::CalibrationMethod` |
| `finstack_valuations::calibration::execute_step_for_tests` | `finstack_valuations::test_utils::calibration::execute_step` (requires `test-utils` feature or tests) |
| Root-level bump helpers (e.g., `calibration::bump_hazard_shift`) | Import from `finstack_valuations::calibration::bumps::{...}` |
| Config re-exports via `calibration::api::schema::{CalibrationConfig, CalibrationMethod, ...}` | Import config types from `finstack_valuations::calibration::{...}` |
| Solver re-exports (`SequentialBootstrapper`, `GlobalFitOptimizer`, `BootstrapTarget`, `GlobalSolveTarget`) | Use the v2 plan engine; solver internals will become crate-private |

All deprecated symbols emit compile-time warnings with details on the replacement path.

---

## Breaking Changes by Phase

### Phase 1: Critical Safety Fixes

**Status**: ✅ Completed (Required for all users)

| Component | Change | Impact | Migration Path |
|-----------|--------|--------|----------------|
| **MetricRegistry::compute()** | Defaults to strict mode | Errors instead of `0.0` for unknown metrics | Add error handling or use `Instrument::price_with_metrics()` |
| **MetricId::parse_strict()** | New strict parser | Rejects unknown metric names | Replace `from_str()` with `parse_strict()` for user inputs |
| **Dependency resolution** | Errors propagated | Circular dependencies detected | Fix circular metric deps in custom calculators |
| **Calibration residuals** | Normalized by notional | Better solver scaling (non-breaking) | No code changes required |

**Severity**: 🔴 Critical - Silent failures could lead to wrong risk reports

### Phase 2: Market Convention Alignment

**Status**: ✅ Completed (Recommended for FX/multi-currency users)

| Component | Change | Impact | Migration Path |
|-----------|--------|--------|----------------|
| **FX spot date calculation** | Uses joint business days | Different spot dates near holidays | Verify FX settlement logic; update tests |
| **Calendar resolution** | Errors on unknown IDs | No silent fallback to `weekends_only` | Handle `CalendarNotFound` error OR use explicit `None` |
| **Swap spread field** | Renamed to `spread_decimal` | Field name change in JSON/API | Update quote construction; use `spread_decimal` field |

**Severity**: 🟠 Major - Incorrect settlement dates and unit confusion

### Phase 3: API Safety & Constructor Removal

**Status**: ✅ Completed (Breaking)

| Component | Change | Impact | Migration Path |
|-----------|--------|--------|----------------|
| **CdsOption constructors** | Removed (panicking) | Compile errors | Use `try_new()` instead of `new()` |
| **Clippy safety lints** | Enabled at crate level | Prevents new panic/expect in code | No user impact; internal only |

**Severity**: 🟠 Major - Removed APIs; migration required

---

## Removed APIs (0.4.1)

The following APIs were removed in **0.4.1**. Use the replacements listed below:

- `CashFlowBuilder::build()` → use `build_with_curves(None)` or `build_with_curves(Some(..))`
- Python `CashFlowBuilder.build()` → use `build_with_curves(None)` or `build_with_curves(market)`
- JS/WASM `CashFlowBuilder.build()` → use `buildWithCurves()` with an optional market context
- `MetricRegistry::compute_best_effort()` → use `compute()` (strict) or `Instrument::price_with_metrics()`
- `Instrument::matches_selector/has_tag/get_meta` → use `instrument.attributes().matches_selector/has_tag/get_meta`
- Binomial tree barrier wrappers (`price_up_and_out`, `price_down_and_out`, `price_up_and_in`, `price_down_and_in`, `price_*_american`) → use `price_barrier_out/in` variants

## Migration Strategies

### Strategy 1: Fast Migration (Breaking Changes Accepted)

**Best for**: New projects, small codebases, teams prioritizing correctness

**Timeline**: 2-4 hours

**Steps**:

1. Update to 0.8.0
2. Fix compiler errors (strict mode, FX calendar errors)
3. Add error handling for new error variants
4. Update tests to expect correct FX spot dates
5. Run full test suite

**Pros**:

- ✅ Immediate safety improvements
- ✅ No technical debt
- ✅ Clean codebase

**Cons**:

- ❌ Requires immediate code changes
- ❌ May need test updates

### Strategy 2: Gradual Migration (Preserve Old Behavior)

**Best for**: Large codebases, production systems, teams with limited migration time

**Timeline**: 1-2 days (spread over multiple releases)

**Steps**:

**Week 1 (Version 0.8.0)**:

1. Update to 0.8.0
2. Add error handling around `compute()` and migrate callers to strict results
3. Add calendar fallback handling where needed
4. Deploy and monitor

**Week 2-4 (Version 0.8.1+)**:
5. Gradually migrate to strict mode module-by-module
6. Add proper error handling incrementally
7. Update FX settlement logic and verify correctness

**Pros**:

- ✅ Non-disruptive deployment
- ✅ Time to test each change
- ✅ Rollback friendly

**Cons**:

- ❌ Temporary technical debt
- ❌ Must schedule follow-up work

### Strategy 3: Mixed Approach (Recommended)

**Best for**: Most teams

**Timeline**: 4-8 hours

**Steps**:

1. **Phase 1 (Critical)**: Immediate strict mode for risk-critical paths
2. **Phase 1 (Non-critical)**: Best effort for reporting/analytics
3. **Phase 2**: Update FX settlement (verify behavior change is correct)
4. **Phase 3**: Update removed constructors to `try_*` variants

**Pros**:

- ✅ Balances safety and pragmatism
- ✅ Focuses on high-risk areas first
- ✅ Gradual for lower-risk code

**Cons**:

- ❌ Requires judgment calls on criticality
- ❌ Mixed patterns in codebase temporarily

---

## Code Examples: Before & After

### Example 1: Metrics Computation (Phase 1)

#### Before (0.7.x) - Silent Failures

```rust
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};

let registry = standard_registry();
let metric_ids = vec![MetricId::Dv01, MetricId::Ytm, MetricId::DurationMod];

let mut context = MetricContext::new(instrument, market, as_of, pv, MetricContext::default_config());

// Problem: Returns 0.0 for unknown/failed metrics without error
let metrics = registry.compute(&metric_ids, &mut context)?;

// User might think DV01 is 0.0 when it actually failed to compute!
let dv01 = metrics.get(&MetricId::Dv01).copied().unwrap_or(0.0);
```

#### After (0.8.0) - Strict Mode (Recommended)

```rust
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};

let registry = standard_registry();
let metric_ids = vec![MetricId::Dv01, MetricId::Ytm, MetricId::DurationMod];

let mut context = MetricContext::new(instrument, market, as_of, pv, MetricContext::default_config());

// Strict mode: errors are returned and must be handled
let metrics = match registry.compute(&metric_ids, &mut context) {
    Ok(m) => m,
    Err(e) => {
        // Handle specific error types
        match e {
            Error::UnknownMetric { metric_id, available } => {
                log::error!("Unknown metric '{}'. Available: {:?}", metric_id, available);
                return Err(e);
            }
            Error::MetricNotApplicable { metric_id, instrument_type } => {
                log::warn!("Metric '{}' not applicable to {}", metric_id, instrument_type);
                // Could filter and retry with applicable metrics
                return Err(e);
            }
            Error::MetricCalculationFailed { metric_id, cause } => {
                log::error!("Failed to compute '{}': {}", metric_id, cause);
                return Err(e);
            }
            _ => return Err(e),
        }
    }
};

// DV01 is guaranteed to be present or function returned early
let dv01 = metrics[&MetricId::Dv01];
```

#### After (0.8.0) - Strict Mode

```rust
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};

let registry = standard_registry();
let metric_ids = vec![MetricId::Dv01, MetricId::Ytm, MetricId::DurationMod];

let mut context = MetricContext::new(instrument, market, as_of, pv, MetricContext::default_config());

// Strict mode: handle errors explicitly
let metrics = registry.compute(&metric_ids, &mut context)?;
let dv01 = metrics[&MetricId::Dv01];
```

### Example 2: Metric Parsing from Config (Phase 1)

#### Before (0.7.x) - Accepts Anything

```rust
use std::str::FromStr;

#[derive(Deserialize)]
struct RiskConfig {
    metrics: Vec<String>,
}

fn load_config(config: &RiskConfig) -> Result<Vec<MetricId>> {
    // Problem: Typos become "custom" metrics silently
    config.metrics
        .iter()
        .map(|s| MetricId::from_str(s))  // Never fails!
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

// Config with typo:
// metrics: ["dv01", "ytm", "duraton_mod"]  # Typo in "duration"
//
// Old behavior: Creates custom metric "duraton_mod" which computes to 0.0
// User doesn't know there's a typo!
```

#### After (0.8.0) - Strict Validation

```rust
use finstack_valuations::metrics::MetricId;

#[derive(Deserialize)]
struct RiskConfig {
    #[serde(deserialize_with = "deserialize_metric_ids")]
    metrics: Vec<MetricId>,
}

fn deserialize_metric_ids<'de, D>(deserializer: D) -> Result<Vec<MetricId>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings = Vec::<String>::deserialize(deserializer)?;

    strings
        .iter()
        .map(|s| {
            // Strict parsing: rejects unknown metrics at load time
            MetricId::parse_strict(s)
                .map_err(|e| D::Error::custom(format!("Invalid metric ID '{}': {}", s, e)))
        })
        .collect()
}

// Config with typo now fails to load with clear error:
// Error: Invalid metric ID 'duraton_mod': UnknownMetric {
//     metric_id: "duraton_mod",
//     available: ["dv01", "cs01", "duration_mod", "duration_mac", ...]
// }
```

### Example 3: FX Spot Date Calculation (Phase 2)

#### Before (0.7.x) - Calendar Days (Incorrect)

```rust
use finstack_valuations::instruments::common::fx_dates::roll_spot_date;

let trade_date = Date::from_ymd(2024, 12, 30)?;  // Monday
let spot_lag = 2;

// Old behavior: adds 2 calendar days → 2025-01-01 (New Year's Day, holiday)
// Then adjusts for business day → 2025-01-02
let spot_date = roll_spot_date(
    trade_date,
    spot_lag,
    BusinessDayConvention::Following,
    Some("nyse"),
    Some("target2"),
)?;

// Result: 2025-01-02 (WRONG: both markets closed on Jan 1,
//                       should skip to first joint business day)
```

#### After (0.8.0) - Joint Business Days (Correct)

```rust
use finstack_valuations::instruments::common::fx_dates::roll_spot_date;

let trade_date = Date::from_ymd(2024, 12, 30)?;  // Monday
let spot_lag = 2;

// New behavior: adds 2 JOINT business days
// Dec 30 (Mon) → Dec 31 (Tue) is business day #1
// Dec 31 (Tue) → Jan 2 (Thu) is business day #2 (Jan 1 skipped - both closed)
let spot_date = roll_spot_date(
    trade_date,
    spot_lag,
    BusinessDayConvention::Following,
    Some("nyse"),
    Some("target2"),
)?;

// Result: 2025-01-02 (CORRECT: first day both markets are open)
```

**Impact**: FX spot dates may be different near holidays. Verify your tests and compare against vendor calendars (Bloomberg, ISDA).

### Example 4: Calendar Resolution Error Handling (Phase 2)

#### Before (0.7.x) - Silent Fallback

```rust
use finstack_valuations::instruments::common::fx_dates::resolve_calendar;

// Typo in calendar ID
let cal = resolve_calendar(Some("target3"));  // Should be "target2"

// Old behavior: silently falls back to weekends_only calendar!
// No error, no warning - just wrong business day logic
```

#### After (0.8.0) - Explicit Error

```rust
use finstack_valuations::instruments::common::fx_dates::resolve_calendar;
use finstack_core::Error;

// Typo in calendar ID
let cal = match resolve_calendar(Some("target3")) {
    Ok(c) => c,
    Err(Error::InputError(e)) if e.contains("Calendar not found") => {
        // Error: CalendarNotFound {
        //     calendar_id: "target3",
        //     hint: "Available calendars: nyse, target2, gblo, jpto, ..."
        // }
        log::error!("Invalid calendar ID: {}", e);
        return Err(e.into());
    }
    Err(e) => return Err(e),
};

// OR: Use explicit None for weekends-only (documents intent)
let weekends_cal = resolve_calendar(None)?;  // OK: weekends-only by design
```

### Example 5: Swap Spread Quote Units (Phase 2)

#### Before (0.7.x) - Ambiguous Field

```rust
use finstack_valuations::market::quotes::RateQuote;

// Was the spread field in decimal or bp? Unclear!
let swap_quote = RateQuote::Swap {
    maturity: Pillar::Tenor(Tenor::Y5),
    fixed_rate: 0.03,           // 3% (clear: decimal)
    spread: Some(0.0010),       // 10 bp? or 0.10 bp? (AMBIGUOUS)
    currency: Currency::USD,
};

// Internal conversion: spread * 10000 → assumed decimal input
// But users might provide bp directly → 10x off!
```

#### After (0.8.0) - Explicit Decimal Field

```rust
use finstack_valuations::market::quotes::RateQuote;

// Field name makes units clear
let swap_quote = RateQuote::Swap {
    maturity: Pillar::Tenor(Tenor::Y5),
    fixed_rate: 0.03,                    // 3% (decimal)
    spread_decimal: Some(0.0010),        // 0.10% = 10 bp (clear: decimal)
    currency: Currency::USD,
};

// Internal conversion: spread_decimal * 10000 → basis points
// Field name documents the contract: input is decimal
```

**Migration Note**: Legacy `"spread"` is no longer accepted. Use `"spread_decimal"` for clarity.

### Example 6: Constructor Removal (Phase 3)

#### Before (0.7.x) - Panicking Constructor

```rust
use finstack_valuations::instruments::credit_derivatives::cds_option::{CdsOption, CdsOptionParams};

// Panics if parameters are invalid (e.g., expiry > maturity)
let params = CdsOptionParams::call(
    100.0,                              // strike_spread_bp
    date!(2025 - 06 - 20),             // expiry
    date!(2030 - 06 - 20),             // cds_maturity
    Money::new(10_000_000.0, Currency::USD),
);

let option = CdsOption::new(
    "CDSOPT-CALL",
    &params,
    &credit,
    "USD-OIS",
    "VOL-SURFACE",
);
// ⚠️ Panics on invalid inputs! No way to handle error gracefully.
```

#### After (0.8.0) - Result-Returning Constructor

```rust
use finstack_valuations::instruments::credit_derivatives::cds_option::{CdsOption, CdsOptionParams};

// Returns Result for error handling
let params = CdsOptionParams::try_call(
    100.0,                              // strike_spread_bp
    date!(2025 - 06 - 20),             // expiry
    date!(2030 - 06 - 20),             // cds_maturity
    Money::new(10_000_000.0, Currency::USD),
)?;  // ✅ Errors can be handled

let option = CdsOption::try_new(
    "CDSOPT-CALL",
    &params,
    &credit,
    "USD-OIS",
    "VOL-SURFACE",
)?;  // ✅ Errors can be handled
```

---

## Error Handling Updates

### New Error Variants (Phase 1)

Add these to your error handling code:

```rust
use finstack_core::Error;

match error {
    // NEW: Unknown metric requested
    Error::UnknownMetric { metric_id, available } => {
        eprintln!("Unknown metric '{}'. Available:", metric_id);
        for m in available.iter().take(10) {
            eprintln!("  - {}", m);
        }
        // Suggestion: validate metric names at config load time
    }

    // NEW: Metric not applicable to instrument
    Error::MetricNotApplicable { metric_id, instrument_type } => {
        eprintln!("Metric '{}' N/A for {}", metric_id, instrument_type);
        // Suggestion: filter metrics by instrument type before compute
    }

    // NEW: Metric calculation failed
    Error::MetricCalculationFailed { metric_id, cause } => {
        eprintln!("Failed to compute '{}': {}", metric_id, cause);
        // Suggestion: check that market data is complete
    }

    // NEW: Circular dependency
    Error::CircularDependency { path } => {
        eprintln!("Circular dependency: {:?}", path);
        // Suggestion: review custom metric calculator dependencies
    }

    // UPDATED: Calendar errors (Phase 2)
    Error::InputError(e) if e.contains("Calendar not found") => {
        eprintln!("Calendar resolution failed: {}", e);
        // Suggestion: check calendar ID spelling, use None for weekends-only
    }

    // ... existing error variants
    _ => { /* handle other errors */ }
}
```

---

## Testing Recommendations

### Phase 1: Metrics Testing

**Unit Tests**:

```rust
#[test]
fn test_strict_mode_unknown_metric() {
    let registry = standard_registry();
    let invalid = vec![MetricId::custom("unknown_metric")];
    let mut ctx = MetricContext::new(/*...*/, MetricContext::default_config());

    // Should error in strict mode
    let result = registry.compute(&invalid, &mut ctx);
    assert!(matches!(
        result,
        Err(Error::UnknownMetric { .. })
    ));
}

```

**Integration Tests**:

```rust
#[test]
fn test_end_to_end_metrics_workflow() {
    // Setup: calibrate curve, create instrument, build market
    let market = build_test_market();
    let instrument = Bond::fixed_semiannual(/*...*/);

    // Compute metrics in strict mode
    let registry = standard_registry();
    let metrics = vec![MetricId::Dv01, MetricId::Convexity, MetricId::DurationMod];

    let pv = instrument.npv(&market, as_of)?;
    let mut ctx = MetricContext::new(&instrument, &market, as_of, pv, MetricContext::default_config());

    let results = registry.compute(&metrics, &mut ctx)?;

    // Verify all requested metrics are present
    assert!(results.contains_key(&MetricId::Dv01));
    assert!(results.contains_key(&MetricId::Convexity));
    assert!(results.contains_key(&MetricId::DurationMod));

    // Verify values are reasonable
    assert!(results[&MetricId::Dv01] > 0.0);
}
```

### Phase 2: FX Settlement Testing

**Golden Tests** (compare against known vendor dates):

```rust
#[test]
fn test_fx_spot_date_christmas_2024() {
    let trade_date = Date::from_ymd(2024, 12, 23)?;  // Monday before Christmas

    // USD/EUR T+2 spot around Christmas
    let spot_date = roll_spot_date(
        trade_date,
        2,  // T+2
        BusinessDayConvention::Following,
        Some("nyse"),
        Some("target2"),
    )?;

    // Expected: Dec 27 (Fri) - both markets open
    // (Dec 24-26 = Christmas holidays)
    assert_eq!(spot_date, Date::from_ymd(2024, 12, 27)?);
}

#[test]
fn test_fx_spot_date_new_year_2025() {
    let trade_date = Date::from_ymd(2024, 12, 30)?;  // Monday before New Year

    let spot_date = roll_spot_date(
        trade_date,
        2,
        BusinessDayConvention::Following,
        Some("nyse"),
        Some("target2"),
    )?;

    // Expected: Jan 2 (Thu) - first joint business day
    // (Jan 1 = New Year's Day, both markets closed)
    assert_eq!(spot_date, Date::from_ymd(2025, 1, 2)?);
}
```

**Regression Tests** (ensure behavior change is correct):

```rust
#[test]
fn test_fx_spot_date_changed_from_v0_7() {
    let trade_date = Date::from_ymd(2024, 12, 30)?;

    let new_spot = roll_spot_date(
        trade_date,
        2,
        BusinessDayConvention::Following,
        Some("nyse"),
        Some("target2"),
    )?;

    // Document the change:
    // v0.7.x: 2025-01-01 (wrong - holiday)
    // v0.8.0: 2025-01-02 (correct - first joint business day)
    assert_eq!(new_spot, Date::from_ymd(2025, 1, 2)?);

    // Add a comment explaining why the change is correct:
    // "New Year's Day (Jan 1) is a holiday on both NYSE and TARGET2.
    //  T+2 from Dec 30 should skip Jan 1 and settle on Jan 2."
}
```

### Phase 3: Constructor Testing

```rust
#[test]
fn test_constructor_error_handling() {
    // Invalid parameters should return error, not panic
    let result = CdsOptionParams::try_call(
        100.0,
        date!(2030 - 06 - 20),  // expiry AFTER maturity (invalid)
        date!(2025 - 06 - 20),  // maturity
        Money::new(1_000_000.0, Currency::USD),
    );

    assert!(result.is_err());

    // Should provide useful error message
    match result {
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("expiry") || msg.contains("maturity"));
        }
        Ok(_) => panic!("Should have errored"),
    }
}
```

---

## FAQ

### General Questions

#### Q: Do I need to migrate immediately?

**A**: It depends on your risk tolerance:

- **Phase 1 (Metrics)**: Strongly recommended for all production systems. Silent failures in risk calculations are unacceptable.
- **Phase 2 (FX)**: Required if you price FX instruments or use multi-currency portfolios. The behavior change fixes incorrect settlement dates.
- **Phase 3 (Constructors)**: Required if you used panicking constructors (now removed).

#### Q: Will this break my existing code?

**A**: Yes, Phase 1 and Phase 2 include breaking changes:

- **Phase 1**: `compute()` may return errors where it previously returned `0.0`
- **Phase 2**: FX spot dates may be different near holidays (correct behavior)
- **Phase 3**: Compile errors if removed constructors are still in use

Use the gradual migration strategy if you need time to adapt.

#### Q: How do I roll back if I encounter issues?

**A**:

- **Recommended**: Add error handling to `compute()` and use `Instrument::price_with_metrics()` for strict pricing + metrics
- **Emergency**: Pin to version 0.7.x in Cargo.toml temporarily
- **Best practice**: Run migrations in a feature branch with comprehensive testing before merging

### Phase 1: Metrics

#### Q: Why did you make strict mode the default?

**A**: Silent failures in financial calculations are a critical safety issue. The previous behavior (returning `0.0` for failures) could lead to:

- Undetected bugs (typos in metric names)
- Wrong risk reports (missing DV01 reported as zero exposure)
- Compliance violations (incomplete risk disclosures)

Best-effort mode has been removed; strict error handling should be the goal.

#### Q: Can I get the old behavior back?

**A**: No. Use explicit error handling with `compute()` or `Instrument::price_with_metrics()` and decide how to handle missing/failed metrics in your application logic.

#### Q: How do I know which metrics are available?

**A**: All standard metrics are listed in `MetricId::ALL_STANDARD`:

```rust
use finstack_valuations::metrics::MetricId;

for metric in MetricId::ALL_STANDARD {
    println!("- {}", metric.as_str());
}
```

Or check the error message when strict parsing fails:

```rust
match MetricId::parse_strict("unknown") {
    Err(Error::UnknownMetric { available, .. }) => {
        println!("Available: {:?}", available);
    }
    _ => {}
}
```

#### Q: What if I need a custom metric?

**A**: Custom metrics are still supported:

```rust
// For programmatic use (you control the name):
let custom = MetricId::custom("my_proprietary_metric");

// OR if parsing from string in controlled context:
let custom = MetricId::from_str("my_proprietary_metric").unwrap();
```

Key difference:

- **`parse_strict()`**: Rejects unknown metrics → use for user inputs (config, CLI)
- **`from_str()` / `custom()`**: Accepts anything → use for programmatic construction

#### Q: My tests are failing with "MetricNotApplicable". What do I do?

**A**: You're requesting a metric that doesn't apply to your instrument. For example:

- `ImpliedVol` on a bond (bonds don't have implied vol)
- `EffectiveSpread` on an option

**Solutions**:

1. **Filter metrics by instrument type**:

   ```rust
   let applicable_metrics = metrics
       .iter()
       .filter(|m| is_applicable_to(m, instrument_type))
       .collect();
   ```

2. **Handle the error and retry**:

   ```rust
   match registry.compute(&all_metrics, &mut ctx) {
       Err(Error::MetricNotApplicable { metric_id, .. }) => {
           // Retry without the non-applicable metric
           let filtered = all_metrics.iter()
               .filter(|&m| m != &metric_id)
               .cloned()
               .collect();
           registry.compute(&filtered, &mut ctx)?
       }
       result => result?,
   }
   ```

### Phase 2: FX Settlement

#### Q: Why did FX spot dates change?

**A**: The old implementation used **calendar days** instead of **joint business days**. This produced incorrect settlement dates when either the base or quote currency had a holiday.

**Example**:

- **Old (wrong)**: Dec 30 trade → add 2 calendar days → Jan 1 (New Year's, closed) → adjust to Jan 2
- **New (correct)**: Dec 30 trade → add 2 joint business days → skip Jan 1 (both markets closed) → Jan 2

The new behavior matches ISDA conventions and vendor calendars (Bloomberg, Reuters).

#### Q: How do I verify my FX spot dates are correct?

**A**: Compare against:

1. **Vendor calendars**: Bloomberg CALD, Reuters calendar
2. **ISDA FX Settlement Calendar**: Official holiday schedules
3. **Golden test files**: See `finstack/valuations/tests/golden/fx_spot_dates.json`

**Example verification**:

```bash
# Run golden tests
cargo test --test integration_tests fx_settlement

# Check specific date:
cargo test --test integration_tests test_usd_eur_spot_christmas_2024 -- --nocapture
```

#### Q: Will this affect my existing trades?

**A**: It depends:

- **Trades already settled**: No impact (historical)
- **Trades priced but not settled**: Spot date may be different (verify correctness)
- **Future trades**: Will use correct joint business day logic

**Recommendation**: Run a comparison report of spot dates before/after upgrade for in-flight trades.

#### Q: What if I need the old behavior temporarily?

**A**: The old behavior was incorrect and should not be used. If you must, you can:

1. Pin to version 0.7.x temporarily
2. Or implement a custom `add_calendar_days()` helper (not recommended)

Better: Update your tests and verify the new behavior is correct against vendor calendars.

### Phase 3: Constructors

#### Q: Why remove `new()` constructors?

**A**: Panicking constructors are unsafe for library APIs because:

1. **Panics can't be caught**: No way to handle errors gracefully in calling code
2. **Lost error context**: Stack unwinding loses detailed validation failure information
3. **FFI safety**: Panics across FFI boundaries (Python, WASM) are undefined behavior
4. **Production risk**: Crashes in pricing engines instead of recoverable errors

#### Q: Do I need to update test code?

**A**: Yes. Use `try_new()` with `expect()`:

```rust
#[test]
fn test_pricing() {
    let option = CdsOption::try_new(...)
        .expect("Valid test parameters");
    // ... test code
}
```

### Gradual Migration

#### Q: How do I migrate a large codebase gradually?

**A**: Recommended 3-phase approach:

**Phase 1**: Add error handling on critical paths and switch those call sites to strict compute:

```rust
mod risk_reporting {
    fn compute_dv01(...) -> Result<f64> {
        let metrics = registry.compute(&[MetricId::Dv01], &mut ctx)?;
        Ok(metrics[&MetricId::Dv01])
    }
}
```

**Phase 2**: Migrate non-critical paths module by module:

```rust
mod analytics {
    fn compute_metrics(...) -> Result<HashMap<MetricId, f64>> {
        registry.compute(&metric_ids, &mut ctx)
    }
}
```

**Phase 3**: Clean up fallback logic and ensure errors are surfaced or handled explicitly.

**Timeline**: Aim for 4-8 weeks total migration time.

#### Q: Can I opt out of strict mode?

**A**: No. Best-effort mode was removed. Use `compute()` with explicit error handling or `Instrument::price_with_metrics()` for combined pricing and metrics.

---

## Support & Resources

### Documentation

- **API Docs**: Run `cargo doc --open` for full rustdoc
- **Migration Guide** (this document): `MIGRATION_GUIDE.md`
- **Crate-specific migration**: `finstack/valuations/MIGRATION.md`
- **Changelog**: `finstack/valuations/CHANGELOG.md`

### Example Code

- **Integration tests**: `finstack/valuations/tests/integration/metrics_strict_mode.rs`
- **FX settlement tests**: `finstack/valuations/tests/integration/fx_settlement.rs`
- **Golden test data**: `finstack/valuations/tests/golden/fx_spot_dates.json`

### Getting Help

- **Issue tracker**: [GitHub Issues](https://github.com/yourusername/finstack/issues)
- **Discussion forum**: [GitHub Discussions](https://github.com/yourusername/finstack/discussions)
- **Email**: For private concerns, email [maintainer@example.com]

### Reporting Problems

When reporting issues, please include:

1. **Version**: Output of `cargo tree | grep finstack`
2. **Error message**: Full error text with stack trace
3. **Minimal example**: Smallest code that reproduces the issue
4. **Migration strategy**: Which strategy you're using (fast/gradual/mixed)

### Contributing

If you find issues with the migration guide or have suggestions:

1. Open an issue with tag `documentation`
2. Submit a PR with improvements
3. Share your migration experience in discussions

---

## Appendix: Version Compatibility Matrix

| Feature | 0.7.x | 0.8.0 | 0.9.0 (planned) | 1.0.0 (planned) |
|---------|-------|-------|-----------------|-----------------|
| **Metrics strict mode** | ❌ Best effort only | ✅ Default strict | ✅ Strict only | ✅ Strict only |
| **Metric parsing strict** | ❌ Permissive | ✅ `parse_strict()` available | ✅ `parse_strict()` | ✅ `parse_strict()` |
| **FX joint business days** | ❌ Calendar days | ✅ Joint business days | ✅ Joint business days | ✅ Joint business days |
| **Calendar error handling** | ❌ Silent fallback | ✅ Explicit error | ✅ Explicit error | ✅ Explicit error |
| **Swap spread field** | `spread` (ambiguous) | `spread_decimal` (clear) | `spread_decimal` | `spread_decimal` |
| **Panicking constructors** | ✅ Available | ❌ Removed | ❌ Removed | ❌ Removed |
| **Calibration residuals** | ❌ Not normalized | ✅ Normalized | ✅ Normalized | ✅ Normalized |
| **Safety lints** | ❌ Not enforced | ⚠️ Enforced (allowed internally) | ✅ Enforced (violations reduced) | ✅ Fully enforced |

---

**Document Version**: 1.0
**Last Updated**: December 20, 2024
**Author**: Finstack Core Team

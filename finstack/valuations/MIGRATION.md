# Migration Guide: Market Convention Refactors (Phases 1-3)

This guide helps you migrate code to use the new **strict metrics mode**, **API safety improvements**, and related changes introduced in the market convention refactors.

**Version**: 0.8.0  
**Date**: December 2024  
**Breaking Changes**: Yes (see details below)

---

## Table of Contents

1. [Overview](#overview)
2. [Breaking Changes Summary](#breaking-changes-summary)
3. [Metrics Framework Changes (Phase 1)](#metrics-framework-changes)
4. [Calibration Improvements (Phase 1)](#calibration-improvements)
5. [Constructor Removals (Phase 3)](#constructor-removals)
6. [Error Handling Updates](#error-handling-updates)
7. [Migration Checklist](#migration-checklist)
8. [FAQ](#faq)

---

## Overview

Phase 1 addresses critical safety issues that could lead to silent errors in risk calculations:

- **Metrics strict mode**: Errors are now surfaced instead of silently returning `0.0`
- **Strict metric parsing**: Unknown metric names are rejected at parse time
- **Calibration fixes**: Residual normalization ensures consistent solver behavior
- **Better error messages**: Detailed error information including circular dependencies

### Key Principle

**Before**: Silent failures and best-effort behavior were the default.  
**After**: Explicit error handling is the default; fallback behavior requires opt-in.

---

## Breaking Changes Summary

| Component | Change | Impact | Migration |
|-----------|--------|--------|-----------|
| `MetricRegistry::compute()` | Now defaults to strict mode | Code expecting silent `0.0` for unknown metrics will fail | Use `compute_best_effort()` for old behavior OR handle errors |
| `MetricId::parse_strict()` | New strict parsing method | User-provided metric names must be validated | Replace `from_str()` with `parse_strict()` for user inputs |
| Dependency resolution | Errors propagated instead of ignored | Circular dependencies now detected | Fix circular metric dependencies in custom calculators |
| Error types | New error variants added | Match arms may need updating | Add new error variants to match statements |

---

## Metrics Framework Changes

### 1. Strict Mode is Now Default

**Before (0.7.x)**:
```rust
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};

let registry = standard_registry();
let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

let mut context = MetricContext::new(instrument, market, as_of, pv);

// This would silently return 0.0 for unknown/failed metrics
let metrics = registry.compute(&metric_ids, &mut context)?;
```

**After (0.8.0)** - **Recommended Approach**:
```rust
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};

let registry = standard_registry();
let metric_ids = vec![MetricId::Dv01, MetricId::Ytm];

let mut context = MetricContext::new(instrument, market, as_of, pv);

// Strict mode is the default - errors are returned, not silently suppressed
let metrics = match registry.compute(&metric_ids, &mut context) {
    Ok(m) => m,
    Err(e) => {
        // Handle error explicitly - could be:
        // - UnknownMetric: metric not registered
        // - MetricNotApplicable: metric doesn't apply to this instrument
        // - MetricCalculationFailed: computation error
        eprintln!("Metric computation failed: {}", e);
        return Err(e);
    }
};
```

**After (0.8.0)** - **Gradual Migration with Best Effort**:
```rust
use finstack_valuations::metrics::core::registry::StrictMode;

// Option 1: Explicit mode parameter (deprecated, will be removed)
let metrics = registry.compute_with_mode(
    &metric_ids,
    &mut context,
    StrictMode::BestEffort
)?;

// Option 2: Convenience method (preferred for gradual migration)
let metrics = registry.compute_best_effort(&metric_ids, &mut context)?;
// Note: This logs warnings for failures and returns 0.0 as fallback
```

### 2. Strict Metric Parsing

**Before (0.7.x)**:
```rust
use std::str::FromStr;

// This would accept ANY string and create a custom metric
let metric = MetricId::from_str("typo_in_name").unwrap();
// metric.as_str() == "typo_in_name" (silently wrong!)
```

**After (0.8.0)** - **For User Inputs**:
```rust
// Use parse_strict() for user-provided metric names
let metric = MetricId::parse_strict("dv01")?;
// ✅ Known metric: returns MetricId::Dv01

let metric = MetricId::parse_strict("typo_in_name");
// ❌ Unknown metric: returns Err(UnknownMetric { metric_id: "typo_in_name", available: [...] })
```

**After (0.8.0)** - **For Programmatic Use**:
```rust
use std::str::FromStr;

// FromStr remains permissive for backwards compatibility in programmatic use
let custom = MetricId::from_str("my_custom_metric").unwrap();
// Still works for custom metrics created programmatically
```

**Recommended**: Use `parse_strict()` for:
- JSON/YAML config files
- CLI arguments
- User-provided strings

Use `from_str()` or direct constructors for:
- Hard-coded metric names in source code
- Custom metric extensions (where you control the name)

### 3. Example: JSON Config Validation

**Before (0.7.x)**:
```rust
#[derive(Deserialize)]
struct RiskConfig {
    #[serde(deserialize_with = "deserialize_metric_id")]
    metrics: Vec<MetricId>,
}

fn deserialize_metric_id<'de, D>(deserializer: D) -> Result<MetricId, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // This would accept anything (typos became custom metrics)
    Ok(MetricId::from_str(&s).unwrap())
}
```

**After (0.8.0)**:
```rust
use serde::de::Error as _;

fn deserialize_metric_id<'de, D>(deserializer: D) -> Result<MetricId, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // Strict parsing rejects typos/unknown metrics at deserialization time
    MetricId::parse_strict(&s)
        .map_err(|e| D::Error::custom(format!("Invalid metric ID: {}", e)))
}
---

## Calibration Improvements

### Residual Normalization Fix

**What Changed**: Global calibration residuals are now normalized by `residual_notional` (matching per-quote residual behavior).

**Impact**: 
- Calibrations with large notionals (e.g., 1,000,000) may see slightly different convergence behavior
- Solver tolerances now have consistent meaning across different notional scales
- No code changes required (internal fix)

**Verification**:
```rust
// Test that calibration is invariant to notional scaling
use finstack_valuations::calibration::targets::discount::DiscountCurveTarget;

// Same quotes with different notionals should produce same curve
let target_1m = DiscountCurveTarget::new_from_instruments(
    "USD-OIS", 
    instruments.clone(), 
    DayCount::Act360, 
    1_000_000.0
);

let target_1 = DiscountCurveTarget::new_from_instruments(
    "USD-OIS", 
    instruments.clone(), 
    DayCount::Act360, 
    1.0
);

// Both should converge to same curve (within tolerance)
```

---

## Constructor Removals

### Panicking Constructors Removed (Phase 3)

**What Changed**: Panicking constructors (those using `.expect()` internally) have been removed. Use the fallible `try_*` APIs instead.

**Removed APIs**:
- `CdsOption::new()` → Use `CdsOption::try_new()?` instead
- `CdsOptionParams::new()` → Use `CdsOptionParams::try_new()?` instead
- `CdsOptionParams::call()` → Use `CdsOptionParams::try_call()?` instead
- `CdsOptionParams::put()` → Use `CdsOptionParams::try_put()?` instead

### Why This Change?

Panicking constructors are unsafe for library APIs because:
1. **Panic propagation**: Panics can't be caught and handled gracefully
2. **Error context loss**: Stack unwinding loses detailed error information
3. **FFI safety**: Panics across FFI boundaries are undefined behavior
4. **Production risk**: Panics in financial code can crash pricing engines

### Migration Examples

#### Example 1: CdsOption Construction

**Before (0.7.x)**:
```rust
use finstack_valuations::instruments::cds_option::{CdsOption, CdsOptionParams};
use finstack_valuations::instruments::CreditParams;

let option_params = CdsOptionParams::call(
    100.0,                              // strike_spread_bp
    date!(2025 - 06 - 20),             // expiry
    date!(2030 - 06 - 20),             // cds_maturity
    Money::new(10_000_000.0, Currency::USD),
);

let credit_params = CreditParams::corporate_standard("CORP", "CORP_HAZARD");

let option = CdsOption::new(
    "CDSOPT-CALL-CORP-5Y",
    &option_params,
    &credit_params,
    "USD-OIS",
    "CDSOPT-VOL",
);
// ⚠️ This will panic if parameters are invalid!
```

**After (0.8.0)** - **Recommended Approach**:
```rust
use finstack_valuations::instruments::cds_option::{CdsOption, CdsOptionParams};
use finstack_valuations::instruments::CreditParams;

// Use try_call() instead of call()
let option_params = CdsOptionParams::try_call(
    100.0,                              // strike_spread_bp
    date!(2025 - 06 - 20),             // expiry
    date!(2030 - 06 - 20),             // cds_maturity
    Money::new(10_000_000.0, Currency::USD),
)?;  // ✅ Errors are returned, not panicked

let credit_params = CreditParams::corporate_standard("CORP", "CORP_HAZARD");

// Use try_new() instead of new()
let option = CdsOption::try_new(
    "CDSOPT-CALL-CORP-5Y",
    &option_params,
    &credit_params,
    "USD-OIS",
    "CDSOPT-VOL",
)?;  // ✅ Errors are returned with full context
```

#### Example 2: Put Option with Error Handling

**Before (0.7.x)**:
```rust
let put_params = CdsOptionParams::put(
    150.0,
    date!(2025 - 12 - 20),
    date!(2030 - 12 - 20),
    Money::new(5_000_000.0, Currency::USD),
);
// ⚠️ Panics if expiry > maturity or other validation fails
```

**After (0.8.0)**:
```rust
let put_params = match CdsOptionParams::try_put(
    150.0,
    date!(2025 - 12 - 20),
    date!(2030 - 12 - 20),
    Money::new(5_000_000.0, Currency::USD),
) {
    Ok(params) => params,
    Err(e) => {
        // ✅ Detailed error with validation failure reason
        eprintln!("Failed to create CDS option params: {}", e);
        // Can log, return error, use fallback, etc.
        return Err(e);
    }
};
```

#### Example 3: Batch Construction with Error Collection

**Before (0.7.x)**:
```rust
let options: Vec<CdsOption> = strikes
    .iter()
    .map(|&strike| {
        let params = CdsOptionParams::call(strike, expiry, maturity, notional);
        CdsOption::new(format!("OPT-{}", strike), &params, &credit, disc, vol)
    })
    .collect();
// ⚠️ First invalid strike causes panic, loses all progress
```

**After (0.8.0)**:
```rust
let options: Result<Vec<CdsOption>> = strikes
    .iter()
    .map(|&strike| {
        let params = CdsOptionParams::try_call(strike, expiry, maturity, notional)?;
        CdsOption::try_new(format!("OPT-{}", strike), &params, &credit, disc, vol)
    })
    .collect();

// ✅ Collect all errors, handle them gracefully
match options {
    Ok(opts) => { /* All options created successfully */ }
    Err(e) => {
        eprintln!("Failed to create option batch: {}", e);
        // Can retry with different parameters, skip invalid strikes, etc.
    }
}
```

### Compile Errors

If you still call removed constructors, you'll see errors like this:

```
error[E0599]: no function or associated item named `new` found for struct `CdsOption` in the current scope
  --> src/main.rs:42:18
   |
42 |     let option = CdsOption::new(...);
   |                  ^^^^^^^^^^^^^^
```

### Test Code Migration

For test code using removed constructors:

**Before (0.7.x)**:
```rust
#[test]
fn test_option_pricing() {
    let params = CdsOptionParams::call(100.0, expiry, maturity, notional);
    let option = CdsOption::new("TEST", &params, &credit, disc, vol);
    
    let pv = option.npv(&market, as_of).unwrap();
    assert!(pv.amount() > 0.0);
}
```

**After (0.8.0)** - **Option A: Use `try_new()` with `.expect()`** (recommended):
```rust
#[test]
fn test_option_pricing() {
    let params = CdsOptionParams::try_call(100.0, expiry, maturity, notional)
        .expect("Valid test parameters");
    let option = CdsOption::try_new("TEST", &params, &credit, disc, vol)
        .expect("Valid test option");
    
    let pv = option.npv(&market, as_of).unwrap();
    assert!(pv.amount() > 0.0);
}
```

```

---

## Error Handling Updates

### New Error Variants

Add these variants to your error handling code:

```rust
use finstack_core::Error;

match error {
    // NEW in 0.8.0: Unknown metric requested
    Error::UnknownMetric { metric_id, available } => {
        eprintln!("Unknown metric '{}'. Available metrics:", metric_id);
        for metric in available.iter().take(10) {
            eprintln!("  - {}", metric);
        }
        // ... error handling
    }
    
    // NEW in 0.8.0: Metric not applicable to instrument type
    Error::MetricNotApplicable { metric_id, instrument_type } => {
        eprintln!("Metric '{}' not applicable to {}", metric_id, instrument_type);
        // ... error handling
    }
    
    // NEW in 0.8.0: Metric calculation failed
    Error::MetricCalculationFailed { metric_id, cause } => {
        eprintln!("Failed to compute '{}': {}", metric_id, cause);
        // ... error handling
    }
    
    // NEW in 0.8.0: Circular dependency in metric graph
    Error::CircularDependency { path } => {
        eprintln!("Circular dependency detected: {:?}", path);
        // ... error handling
    }
    
    // ... existing error variants
    _ => { /* ... */ }
}
```

---

## Migration Checklist

Use this checklist to ensure a complete migration:

### For Application Code

- [ ] **Audit metric computation calls**: Find all uses of `registry.compute()`
- [ ] **Choose migration strategy**:
  - [ ] Option A (recommended): Update to handle errors explicitly → use `compute()` with error handling
  - [ ] Option B (gradual): Use `compute_best_effort()` temporarily + add TODO to migrate later
- [ ] **Update user-facing metric parsing**:
  - [ ] Replace `MetricId::from_str()` with `MetricId::parse_strict()` for config/CLI inputs
  - [ ] Keep `from_str()` for hard-coded metric names
- [ ] **Update error handling**: Add match arms for new error variants
- [ ] **Add tests**: Verify error handling for unknown metrics, circular dependencies

### For Library Code

- [ ] **Never suppress errors**: Don't default to `compute_best_effort()` in library paths
- [ ] **Document mode choices**: If offering both modes, document when to use each
- [ ] **Propagate errors**: Use `?` operator, don't convert errors to `0.0` silently
- [ ] **Validate dependencies**: Check for circular dependencies in custom metric calculators

### Testing

- [ ] **Test error paths**: Add tests that verify errors are returned (not silent failures)
- [ ] **Test unknown metrics**: Verify `parse_strict()` rejects typos
- [ ] **Test circular dependencies**: If using custom metrics, verify no cycles exist
- [ ] **Regression tests**: Verify existing metrics still compute correctly

---

## FAQ

### Q: Why did you make strict mode the default? This breaks my code!

**A**: Silent failures in risk calculations are unacceptable for production financial systems. The previous behavior (returning `0.0` for unknown/failed metrics) could lead to:
- Undetected bugs (typos in metric names)
- Wrong risk reports (missing DV01 reported as `0.0` exposure)
- Compliance issues (incomplete risk disclosures)

The best-effort fallback is still available via `compute_best_effort()` for gradual migration, but we strongly recommend migrating to strict error handling.

### Q: Can I get the old behavior back?

**A**: Yes, use `compute_best_effort()`:

```rust
let metrics = registry.compute_best_effort(&metric_ids, &mut context)?;
```

This will:
- Insert `0.0` for unknown/failed metrics
- Log warnings (visible with `RUST_LOG=warn`)
- Return `Ok` instead of `Err`

**Warning**: This should only be a temporary migration step. We recommend migrating to proper error handling.

### Q: How do I know which metrics are available?

**A**: All standard metrics are listed in `MetricId::ALL_STANDARD`:

```rust
use finstack_valuations::metrics::core::ids::MetricId;

for metric in MetricId::ALL_STANDARD {
    println!("- {}", metric.as_str());
}
```

You can also get the list from the error message when strict parsing fails:

```rust
match MetricId::parse_strict("unknown") {
    Ok(_) => {}
    Err(Error::UnknownMetric { available, .. }) => {
        println!("Available metrics: {:?}", available);
    }
    Err(e) => { /* ... */ }
}
```

### Q: What if I need a custom metric?

**A**: You can still create custom metrics programmatically:

```rust
// For hard-coded custom metrics (you control the name):
let custom = MetricId::custom("my_custom_metric");

// OR if parsing from string in controlled context:
let custom = MetricId::from_str("my_custom_metric").unwrap();
```

The difference is:
- **`parse_strict()`**: Rejects unknown metrics → use for user inputs
- **`from_str()` / `custom()`**: Accepts anything → use for programmatic construction

### Q: My tests are failing with "MetricNotApplicable" errors. What do I do?

**A**: This means you're requesting a metric that doesn't apply to your instrument type. For example:
- Requesting `ImpliedVol` on a bond (bonds don't have implied vol)
- Requesting `EffectiveSpread` on an option (not applicable)

**Fix**: Only request metrics that are applicable to your instrument type, or handle the error:

```rust
let metrics = match registry.compute(&all_metrics, &mut context) {
    Ok(m) => m,
    Err(Error::MetricNotApplicable { metric_id, .. }) => {
        // Filter out non-applicable metrics and retry
        let applicable = all_metrics.iter()
            .filter(|&m| m != &metric_id)
            .cloned()
            .collect::<Vec<_>>();
        registry.compute(&applicable, &mut context)?
    }
    Err(e) => return Err(e),
};
```

### Q: How do I migrate a large codebase gradually?

**Recommended approach**:

1. **Phase 1**: Switch to `compute_best_effort()` everywhere (non-breaking)
   ```rust
   // Temporary: preserve old behavior
   let metrics = registry.compute_best_effort(&metric_ids, &mut context)?;
   ```

2. **Phase 2**: Add error handling incrementally
   ```rust
   // Add proper error handling module by module
   let metrics = registry.compute(&metric_ids, &mut context)
       .map_err(|e| format!("Risk calculation failed: {}", e))?;
   ```

3. **Phase 3**: Remove all `compute_best_effort()` calls (complete migration)

### Q: Will future releases break compatibility again?

**A**: Phase 1 makes one intentional breaking change (strict mode default) to fix critical safety issues. Future phases focus on:
- **Phase 2**: Market convention fixes (FX settlement, quote units) - may require config updates but not code changes
- **Phase 3**: API safety (panicking constructors removed) - compile errors enforce migration

Most future breaking changes will follow a deprecation cycle:
1. Deprecate old API in minor release (0.x)
2. Document migration path
3. Remove in the next major release (1.0)

Safety-related removals may be accelerated if they reduce panic/FFI risk.

---

## Safety Lints and Technical Debt (Phase 3.2)

### Clippy Safety Lints Enabled

As of version 0.8.0, the following strict safety lints are enabled at the crate level:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
```

These lints prevent introduction of new panicking code paths in production.

### Current State (December 2024)

**The crate has temporary `#![allow(...)]` attributes** to avoid blocking compilation during the migration period.

**Current violations identified:**
- ~199 uses of `expect()` on `Result` and `Option` values
- 2 uses of `panic!()` macro in production code paths

These violations exist in legacy code and are tracked for remediation in version 1.0.0.

### Migration Timeline

| Phase | Target | Status |
|-------|--------|--------|
| **Step 3.2** (Dec 2024) | Enable lints with temporary allows | ✅ Complete |
| **Version 0.9.0** (Q1 2025) | Fix violations in critical paths (pricing, calibration) | 🚧 Planned |
| **Version 1.0.0** (Q2 2025) | Remove all allows, full compliance | 📋 Planned |

### What This Means for Your Code

**If you use the public API**: No immediate impact. The allows are internal to the crate.

**If you contribute to the crate**: New code MUST NOT use:
- `expect()` - use proper error propagation with `Result<T, E>`
- `panic!()` - use recoverable error handling
- `unwrap()` - already denied, use `?` operator or explicit error handling

**Example - Bad (will be rejected in code review)**:
```rust
pub fn calculate_risk(value: f64) -> f64 {
    let result = compute_dv01(value)
        .expect("DV01 calculation should never fail");  // ❌ Bad
    result
}
```

**Example - Good (required pattern)**:
```rust
pub fn calculate_risk(value: f64) -> Result<f64> {
    let result = compute_dv01(value)
        .map_err(|e| Error::RiskCalculationFailed {
            cause: Box::new(e),
            context: "DV01 calculation".into(),
        })?;  // ✅ Good
    Ok(result)
}
```

### Tracking and Remediation

**Where are the violations?**

Most violations are in:
1. **Instrument constructors** (~50 violations)
   - Many removed as part of the panicking constructor cleanup
   - Remaining internal constructors are being refactored to use `Result`
2. **Internal calibration helpers** (~70 violations)
   - Documented invariants (e.g., "params non-empty, checked above")
   - Being refactored to use Result propagation
3. **Pricing model internals** (~40 violations)
   - Numerical code with performance-critical paths
   - Being replaced with checked operations
4. **Test/unreachable code** (~39 violations)
   - Panic in "should never happen" branches
   - Being replaced with proper error types

**How to help**: See [`finstack/valuations/src/lib.rs`](src/lib.rs) for the TODO tracking this work. Issues tagged with `safety-lints` track specific modules.

---

## Additional Resources

- **API Documentation**: See rustdoc for `MetricRegistry`, `MetricId`, and error types
- **Examples**: See `tests/integration/metrics_strict_mode.rs` for end-to-end examples
- **Issue Tracking**: Report migration issues at [project issue tracker]

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.8.0 | Dec 2024 | Initial Phase 1 release (strict mode, parsing, calibration fix) |
| 0.7.x | - | Legacy behavior (best-effort default, silent failures) |

---

**Questions?** Open an issue or contact the maintainers.

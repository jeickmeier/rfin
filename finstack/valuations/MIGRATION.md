# Migration Guide: Market Convention Refactors (Phase 1)

This guide helps you migrate code to use the new **strict metrics mode** and related safety improvements introduced in Phase 1 of the market convention refactors.

**Version**: 0.8.0  
**Date**: December 2024  
**Breaking Changes**: Yes (see details below)

---

## Table of Contents

1. [Overview](#overview)
2. [Breaking Changes Summary](#breaking-changes-summary)
3. [Metrics Framework Changes](#metrics-framework-changes)
4. [Calibration Improvements](#calibration-improvements)
5. [Error Handling Updates](#error-handling-updates)
6. [Migration Checklist](#migration-checklist)
7. [FAQ](#faq)

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
```

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
- **Phase 3**: API safety (remove panicking constructors) - gradual via deprecation warnings

All future breaking changes will follow a deprecation cycle:
1. Deprecate old API in minor release (0.x)
2. Document migration path
3. Remove in next major release (1.0)

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

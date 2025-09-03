# DiscountCurve Enhancements: Extrapolation & Validation

## Overview

Enhanced the `DiscountCurve` implementation with explicit extrapolation policy configuration and monotonic discount factor validation, addressing critical requirements for robust term structure modeling and credit curve construction.

## Key Enhancements

### 1. Explicit Extrapolation Policy Configuration

**Problem**: Previously, interpolators used implicit boundary clamping without configurable extrapolation behavior.

**Solution**: Added `ExtrapolationPolicy` enum with two modes:

- **FlatZero** (default): Extends endpoint values (traditional, conservative approach)
- **FlatForward**: Extends forward rates from the terminal segments (maintains rate continuity)

**API Changes**:
```rust
// New extrapolation policy enum
pub enum ExtrapolationPolicy {
    FlatZero,     // Extend endpoint values
    FlatForward,  // Extend forward rates
}

// Enhanced DiscountCurve builder
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base_date)
    .knots([(0.0, 1.0), (5.0, 0.88)])
    .monotone_convex()
    .flat_forward_extrapolation()  // or .extrapolation(ExtrapolationPolicy::FlatForward)
    .build()?;
```

### 2. Monotonic Discount Factor Validation

**Problem**: Credit curves require strictly decreasing discount factors to prevent arbitrage opportunities.

**Solution**: Added optional monotonic validation via `.require_monotonic()` builder method.

**API Changes**:
```rust
// Credit curve with monotonic validation
let credit_curve = DiscountCurve::builder("CREDIT-5Y")
    .base_date(base_date)
    .knots(survival_probability_knots)
    .monotone_convex()
    .flat_forward_extrapolation()
    .require_monotonic()  // Critical for credit curves
    .build()?;
```

### 3. Enhanced InterpFn Trait

**Problem**: Interpolators lacked configurable extrapolation behavior.

**Solution**: Extended `InterpFn` trait with extrapolation methods:

```rust
pub trait InterpFn: Send + Sync + core::fmt::Debug {
    fn interp(&self, x: F) -> F;
    fn interp_prime(&self, x: F) -> F;
    
    // New extrapolation methods
    fn set_extrapolation_policy(&mut self, policy: ExtrapolationPolicy);
    fn extrapolation_policy(&self) -> ExtrapolationPolicy;
}
```

### 4. Updated All Interpolation Methods

Enhanced all concrete interpolators with extrapolation support:

- **LinearDf**: Linear extension of terminal segments
- **LogLinearDf**: Maintains constant zero rates in extrapolation regions
- **MonotoneConvex**: Uses Hagan-West polynomial extrapolation with linear approximation
- **CubicHermite**: Extends using terminal slopes from PCHIP algorithm
- **FlatFwd**: Leverages LogLinearDf for consistent behavior

### 5. Python Bindings Integration

**New Python API**:
```python
from finstack.market_data import DiscountCurve, InterpStyle, ExtrapolationPolicy

# Standard discount curve
curve = DiscountCurve(
    id="USD-OIS",
    base_date=Date(2025, 1, 1),
    times=[0.0, 1.0, 2.0, 5.0],
    discount_factors=[1.0, 0.98, 0.95, 0.88],
    interpolation=InterpStyle.MonotoneConvex,
    extrapolation=ExtrapolationPolicy.FlatForward
)

# Credit curve with validation
credit_curve = DiscountCurve(
    id="CREDIT-CORP",
    base_date=Date(2025, 1, 1),
    times=[0.0, 1.0, 3.0, 5.0],
    discount_factors=[1.0, 0.992, 0.970, 0.940],
    interpolation=InterpStyle.MonotoneConvex,
    extrapolation=ExtrapolationPolicy.FlatForward,
    require_monotonic=True  # Critical for credit
)
```

## Technical Implementation Details

### Extrapolation Algorithms

#### Flat-Zero Extrapolation
- **Left**: Returns `df[0]` for `t < t[0]`
- **Right**: Returns `df[n-1]` for `t > t[n-1]`
- **Derivative**: Zero (constant extrapolation)

#### Flat-Forward Extrapolation
- **Left**: Extends the forward rate from the first segment
- **Right**: Extends the forward rate from the last segment
- **Mathematical**: Maintains `f(t) = -d/dt[ln(DF(t))]` constant

### Monotonic Validation

Validates that discount factors satisfy:
```
DF[i+1] ≤ DF[i] for all i
```

This ensures:
- No arbitrage opportunities
- Positive forward rates
- Consistent credit pricing

### Error Handling

Enhanced error reporting:
- `TooFewPoints`: Less than 2 knot points
- `NonMonotonicKnots`: Time points not strictly increasing
- `NonPositiveValue`: Discount factors ≤ 0
- `Invalid`: Non-monotonic discount factors when validation required

## Use Case Guidelines

### Standard Rate Curves
```rust
let ois_curve = DiscountCurve::builder("USD-OIS")
    .knots(market_quotes)
    .monotone_convex()
    .flat_zero_extrapolation()  // Conservative
    .build()?;
```

### Credit Curves
```rust
let credit_curve = DiscountCurve::builder("CREDIT-5Y")
    .knots(survival_probabilities)
    .monotone_convex()
    .flat_forward_extrapolation()  // Market-consistent
    .require_monotonic()           // Prevent arbitrage
    .build()?;
```

### Bootstrapping Applications
```rust
let bootstrap_curve = DiscountCurve::builder("BOOTSTRAP")
    .knots(preliminary_points)
    .log_df()                     // Stable for iteration
    .flat_forward_extrapolation() // Smooth continuation
    .build()?;
```

## Testing Coverage

Comprehensive test suite covering:

1. **Extrapolation Behavior**: Validates both flat-zero and flat-forward policies
2. **Monotonic Validation**: Tests success/failure cases for credit curves
3. **Cross-Interpolation Consistency**: Ensures all methods agree at knot points
4. **Edge Cases**: Minimal curves, zero times, boundary conditions
5. **Credit Curve Construction**: Real-world CDS-to-survival probability workflow

## Performance Considerations

- **Zero Impact**: Extrapolation logic only activates for out-of-bounds queries
- **Minimal Overhead**: Policy stored as simple enum, no runtime dispatch
- **Backward Compatible**: Existing code continues to work with default policies

## Migration Guide

### Existing Code
No changes required - all existing code continues to work with default `FlatZero` extrapolation.

### New Features
```rust
// Enable new features with builder methods
.extrapolation(ExtrapolationPolicy::FlatForward)
.require_monotonic()  // For credit curves
```

### Python Updates
```python
# Optional parameters with sensible defaults
curve = DiscountCurve(
    ...,
    extrapolation=ExtrapolationPolicy.FlatForward,  # Optional
    require_monotonic=True  # Optional, False by default
)
```

## Examples

See:
- `/examples/python/discount_curve_extrapolation_example.py` - Comprehensive demonstration
- `/finstack/core/tests/discount_curve_extrapolation.rs` - Test suite with edge cases

## Mathematical Notes

### Flat-Forward Extrapolation Mathematics

For log-linear interpolation with flat-forward extrapolation:

**Interior**: `DF(t) = exp(y₀ + w(y₁ - y₀))` where `w = (t-t₀)/(t₁-t₀)`

**Right Extrapolation**: `DF(t) = DF(tₙ) * exp(-fₙ * (t - tₙ))`

Where `fₙ` is the instantaneous forward rate from the last segment.

### Monotonic Validation Impact

When `require_monotonic = true`:
- Validates `DF[i+1] ≤ DF[i]` during construction
- Ensures positive forward rates: `f(t) = (DF[i] - DF[i+1])/(Δt * DF[i+1]) ≥ 0`
- Critical for credit curves where increasing survival probability implies negative default intensity

## Future Enhancements

Potential future improvements:
1. **Custom Extrapolation Functions**: User-defined extrapolation logic
2. **Asymptotic Behavior**: Long-term rate convergence models
3. **Multi-Currency Extrapolation**: Currency-specific default policies
4. **Performance Optimizations**: Caching for repeated extrapolation queries

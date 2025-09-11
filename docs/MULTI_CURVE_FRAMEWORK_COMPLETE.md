# Multi-Curve Framework Implementation

## Summary

Successfully implemented a post-2008 multi-curve framework in finstack that enforces strict separation between discount and forward curves, with fallback compatibility for single-curve mode.

## Key Components Implemented

### 1. Multi-Curve Configuration (`multi_curve_mode.rs`)

- **MultiCurveMode enum**: Distinguishes between MultiCurve (post-2008) and SingleCurve (pre-2008) modes
- **MultiCurveConfig struct**: Configuration with:
  - Mode selection (multi-curve vs single-curve)
  - Basis calibration flag
  - Single-curve tenor specification
  - Strict separation enforcement

### 2. Updated Discount Curve Calibrator

Modified `DiscountCurveCalibrator` to respect multi-curve configuration:
- In **MultiCurve mode**: Only creates discount curves, no forward curve derivation
- In **SingleCurve mode**: Optionally derives forward curve from discount curve (legacy compatibility)
- Properly handles basis swap quotes based on mode

### 3. Basis Swap Implementation (`basis_swap.rs`)

Complete basis swap instrument for tenor basis trading:
- **BasisSwapLeg**: Specification for each floating leg
- **BasisSwap**: Full instrument with:
  - Dual floating legs with different tenors
  - Optional spread on either leg
  - Proper multi-curve pricing using separate forward curves
  - OIS discounting

### 4. Framework Integration

- Updated `SimpleCalibration` to use multi-curve configuration
- Forward curves calibrated independently from discount curves
- Basis swaps properly priced using respective forward curves

## Design Principles

### Post-2008 Multi-Curve (Default)
```rust
// Strict separation enforced:
- OIS curves for discounting only
- Forward curves (3M, 6M) calibrated independently
- Basis swaps capture tenor spreads
- No implicit forward curve derivation
```

### Pre-2008 Single-Curve (Fallback)
```rust
// Legacy compatibility mode:
- Discount curve = Forward curve
- Simplified modeling
- Explicit opt-in required via configuration
```

## Usage Example

```rust
// Multi-curve mode (default)
let multi_config = MultiCurveConfig::multi_curve();

// Single-curve mode (fallback)
let single_config = MultiCurveConfig::single_curve(0.25); // 3M tenor

// Apply to calibration
let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    .with_multi_curve_config(multi_config);
```

## Benefits

1. **Market Standard Compliance**: Aligns with post-2008 best practices
2. **Risk Accuracy**: Properly captures tenor basis risk
3. **Flexibility**: Supports both modern multi-curve and legacy single-curve approaches
4. **Explicit Control**: Clear configuration prevents accidental methodology mixing

## Testing

Created comprehensive example (`multi_curve_framework_example.rs`) demonstrating:
- Multi-curve mode configuration
- Single-curve fallback mode
- Basis swap pricing with tenor spreads

## Files Modified/Created

- `finstack/valuations/src/calibration/multi_curve_mode.rs` (new)
- `finstack/valuations/src/calibration/bootstrap/discount.rs` (modified)
- `finstack/valuations/src/calibration/bootstrap/forward_curve.rs` (modified)
- `finstack/valuations/src/calibration/simple_calibration.rs` (modified)
- `finstack/valuations/src/instruments/fixed_income/basis_swap.rs` (new)
- `finstack/valuations/src/instruments/fixed_income/mod.rs` (modified)
- `finstack/valuations/src/calibration/mod.rs` (modified)
- `examples/rust/multi_curve_framework_example.rs` (new)

## Next Steps

Potential enhancements:
1. Add more sophisticated basis swap calibration algorithms
2. Implement cross-currency basis swaps
3. Add OIS-LIBOR transition tools
4. Enhance forward curve interpolation for basis preservation

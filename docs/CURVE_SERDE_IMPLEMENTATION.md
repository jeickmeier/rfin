# Curve Serialization Implementation

## Overview
This PR implements serialization support for curve types that contain interpolators (DiscountCurve, ForwardCurve, InflationCurve), enabling persistence and round-trip accuracy for all interpolation styles.

## Implementation Details

### 1. Interpolator Serialization Infrastructure

#### Added `InterpData` enum (`finstack/core/src/math/interp/types.rs`)
- Serializable representation of the `Interp` enum
- Stores knots, values, and extrapolation policy for each interpolation style
- Supports all five interpolation styles: Linear, LogLinear, MonotoneConvex, CubicHermite, FlatFwd

#### Added accessor methods to interpolator types
- `knots()` - Returns the knot points
- `values()` - Returns the values (with special handling for LogLinearDf which stores log values)
- `extrapolation()` - Returns the extrapolation policy

#### Added conversion methods to `Interp` enum
- `to_interp_data()` - Extracts data for serialization
- `from_interp_data()` - Reconstructs interpolator from serialized data

### 2. Curve Type Serialization

#### DiscountCurve (`finstack/core/src/market_data/term_structures/discount_curve.rs`)
- Added `DiscountCurveData` struct for serializable representation
- Custom `Serialize` and `Deserialize` implementations
- Preserves: curve ID, base date, knots, discount factors, interpolation style, and extrapolation policy

#### ForwardCurve (`finstack/core/src/market_data/term_structures/forward_curve.rs`)
- Added `ForwardCurveData` struct for serializable representation
- Custom `Serialize` and `Deserialize` implementations
- Preserves: curve ID, base date, reset lag, day count, tenor, knots, forward rates, and interpolation style

#### InflationCurve (`finstack/core/src/market_data/term_structures/inflation.rs`)
- Added `InflationCurveState` struct for serializable representation
- Custom `Serialize` and `Deserialize` implementations
- Preserves: curve ID, base CPI, knots, CPI levels, and interpolation style

### 3. Special Handling

#### LogLinearDf and FlatFwd
- LogLinearDf stores logarithms of discount factors internally
- Serialization converts back to original values for consistency
- FlatFwd delegates to its internal LogLinearDf implementation

#### MonotoneConvex Requirements
- Requires non-increasing values (suitable for discount factors)
- Not suitable for forward rates or CPI levels which typically increase
- Tests specifically exclude MonotoneConvex for ForwardCurve and InflationCurve

### 4. Testing

#### Comprehensive Test Suite (`finstack/core/tests/test_curve_serde.rs`)
- Tests all interpolation styles for each curve type (where appropriate)
- Verifies round-trip accuracy to machine precision (< 1e-12)
- Tests extrapolation policy preservation
- Tests both JSON and pretty JSON serialization
- Includes edge cases and boundary conditions

#### Test Coverage
- ✅ DiscountCurve with all 5 interpolation styles
- ✅ ForwardCurve with 4 suitable interpolation styles
- ✅ InflationCurve with 4 suitable interpolation styles
- ✅ Extrapolation policies (FlatZero and FlatForward)
- ✅ Accuracy verification at knot points and interpolated values

### 5. Python Example

Created `examples/python/curve_serialization_example.py` demonstrating:
- Creating curves with various interpolation styles
- Serializing to JSON
- Deserializing and verifying accuracy
- Saving curves to files for persistence

## Benefits

1. **Persistence**: Curves can now be saved to disk and restored later
2. **Interoperability**: JSON format enables exchange between systems
3. **Caching**: Calibrated curves can be cached and reused
4. **Debugging**: Human-readable JSON format aids in debugging
5. **Testing**: Enables golden test files for regression testing

## Usage Example

```rust
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::interp::InterpStyle;

// Create a curve
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(date)
    .knots([(0.0, 1.0), (5.0, 0.88)])
    .set_interp(InterpStyle::MonotoneConvex)
    .build()?;

// Serialize to JSON
let json = serde_json::to_string(&curve)?;

// Deserialize back
let restored: DiscountCurve = serde_json::from_str(&json)?;

// Interpolation accuracy is preserved
assert!((curve.df(2.5) - restored.df(2.5)).abs() < 1e-12);
```

## Files Modified

- `finstack/core/src/math/interp/types.rs` - Added InterpData and conversion methods
- `finstack/core/src/math/interp/linear.rs` - Added accessor methods
- `finstack/core/src/math/interp/log_linear.rs` - Added accessor methods with value conversion
- `finstack/core/src/math/interp/monotone_convex.rs` - Added accessor methods
- `finstack/core/src/math/interp/cubic_hermite.rs` - Added accessor methods
- `finstack/core/src/math/interp/flat_fwd.rs` - Added accessor methods delegating to LogLinearDf
- `finstack/core/src/market_data/term_structures/discount_curve.rs` - Added serialization support
- `finstack/core/src/market_data/term_structures/forward_curve.rs` - Added serialization support
- `finstack/core/src/market_data/term_structures/inflation.rs` - Added serialization support

## Files Added

- `finstack/core/tests/test_curve_serde.rs` - Comprehensive test suite
- `examples/python/curve_serialization_example.py` - Python usage example
- `docs/CURVE_SERDE_IMPLEMENTATION.md` - This documentation

## Testing

All tests pass:
```bash
cargo test --package finstack-core --test test_curve_serde --features serde
# 11 tests, all passing
```

Linting passes:
```bash
make lint
# All checks passed!
```

## Future Enhancements

1. Add binary serialization support (e.g., bincode) for more compact storage
2. Add compression support for large curve collections
3. Consider adding versioning to the serialized format for forward compatibility
4. Add serialization for other curve types (HazardCurve, BaseCorrelationCurve)

# Property Tests for Finite Difference Metrics

This document summarizes the property-based tests added to verify finite difference accuracy and edge case handling in the metrics framework.

## Overview

The property tests are located in `src/metrics/property_tests.rs` and use the `proptest` library to verify fundamental mathematical properties and edge case handling across a wide range of inputs.

## Test Categories

### 1. Central Difference Properties (12 tests)

#### Symmetry Tests
- **`test_central_diff_symmetry`**: Verifies that swapping up/down evaluations negates the result
- **`test_central_mixed_symmetry`**: Verifies Schwarz's theorem for mixed partial derivatives

#### Linearity Tests
- **`test_central_diff_linearity`**: Verifies that derivative(k*f) = k * derivative(f)
- **`test_central_diff_linear_function_invariance`**: Verifies that linear functions give exact derivatives regardless of bump size

#### Accuracy Tests
- **`test_central_diff_quadratic_accuracy`**: Verifies accuracy for quadratic functions with known analytical derivatives
- **`test_central_mixed_bilinear_exact`**: Verifies exact mixed derivatives for bilinear functions

#### Convergence Tests
- **`test_central_diff_convergence`**: Verifies that smaller bumps converge to analytical derivatives
- **`test_central_diff_error_order`**: Verifies O(h²) error scaling for central differences

#### Validation Tests
- **`test_central_diff_rejects_invalid_bumps`**: Verifies rejection of zero/negative bumps
- **`test_central_diff_rejects_non_finite_bumps`**: Verifies rejection of NaN/infinite bumps
- **`test_central_mixed_rejects_invalid_bumps`**: Verifies validation of mixed derivative bumps

#### Stability Tests
- **`test_central_diff_determinism`**: Verifies repeated calculations give identical results
- **`test_central_diff_precision_loss`**: Verifies minimal precision loss for well-conditioned problems

### 2. Adaptive Bump Properties (9 tests)

#### Bounds Tests
- **`test_adaptive_spot_bump_bounds`**: Verifies spot bumps stay within [0.1%, 5%] range
- **`test_adaptive_vol_bump_bounds`**: Verifies vol bumps stay within [0.1%, 5%] range
- **`test_adaptive_bumps_extreme_vol`**: Tests extreme volatility values (0.001 to 5.0)
- **`test_adaptive_bumps_extreme_time`**: Tests extreme time to expiry (0.001 to 100 years)

#### Override Tests
- **`test_adaptive_spot_bump_override`**: Verifies manual overrides are respected exactly
- **`test_adaptive_vol_bump_override`**: Verifies vol bump overrides work correctly
- **`test_adaptive_rate_bump_override`**: Verifies rate bump overrides or defaults

#### Monotonicity Tests
- **`test_adaptive_spot_bump_vol_monotonicity`**: Verifies bumps increase (or cap) with volatility
- **`test_adaptive_spot_bump_time_monotonicity`**: Verifies bumps increase (or cap) with time
- **`test_adaptive_vol_bump_monotonicity`**: Verifies vol bumps scale with current volatility

### 3. Bump Helper Properties (5 tests)

#### Scaling Tests
- **`test_bump_scalar_price_scaling`**: Verifies price bumping scales correctly (with Money rounding tolerance)
- **`test_bump_unitless_scaling`**: Verifies unitless value bumping scales exactly

#### Composition Tests
- **`test_double_bump_composition`**: Verifies that sequential bumps compose correctly
- **`test_zero_bump_identity`**: Verifies zero bumps preserve values (with Money rounding tolerance)

### 4. Edge Case Tests (9 tests)

#### Extreme Value Tests
- **`test_central_diff_tiny_values`**: Tests values in [-1e-10, 1e-10] range
- **`test_central_diff_large_values`**: Tests values in [-1e10, 1e10] range
- **`test_extreme_prices`**: Tests spot prices from 0.0001 to 1e10

#### Zero Input Tests
- **`test_zero_volatility`**: Verifies adaptive bumps work with zero volatility
- **`test_zero_time_to_expiry`**: Verifies adaptive bumps work at expiry
- **`test_central_diff_flat_function`**: Verifies flat functions give zero derivative

#### Numerical Precision Tests
- **`test_very_small_bump`**: Tests bumps near machine precision (1e-8)
- **`test_bump_order_absolute_value`**: Verifies ordering doesn't affect absolute derivative

## Key Mathematical Properties Verified

### 1. **Symmetry**
   - Central difference is anti-symmetric: f'(h) = -f'(-h)
   - Mixed partials commute: ∂²f/∂x∂y = ∂²f/∂y∂x

### 2. **Linearity**
   - Derivative operator is linear: (af + bg)' = af' + bg'
   - Linear functions have exact constant derivatives

### 3. **Convergence**
   - Central differences converge with O(h²) error
   - Smaller bumps give more accurate derivatives (until roundoff)

### 4. **Adaptive Bumps**
   - Bumps scale appropriately with volatility: bump ∝ σ√T
   - Bumps stay within safe bounds [0.1%, 5%]
   - Manual overrides take precedence

### 5. **Composition**
   - Sequential bumps: bump(bump(x, a), b) = bump(x, (1+a)(1+b)-1)
   - Identity: bump(x, 0) ≈ x (within rounding tolerance)

## Implementation Details

### Tolerance Handling

Different test categories use different tolerances based on expected precision:

- **Pure floating-point operations**: 1e-10 relative error
- **Linear function derivatives**: 1e-6 relative error (to account for floating-point arithmetic)
- **Money type operations**: 0.01-0.02 absolute error (cent-level rounding)
- **Quadratic/cubic derivatives**: Adaptive tolerance based on function coefficients

### Property Test Configuration

- Tests use `proptest` with default configuration (256 cases per test)
- Regression files stored in `proptest-regressions/metrics/property_tests.txt`
- Tests are deterministic and reproducible from saved regression cases

## Coverage Summary

The property tests provide comprehensive verification of:

✅ **Mathematical correctness**: Core finite difference formulas
✅ **Numerical stability**: Precision, convergence, determinism
✅ **Edge case handling**: Zero, extreme, and boundary values
✅ **Adaptive algorithms**: Bump size selection and scaling
✅ **Input validation**: Rejection of invalid inputs
✅ **Integration**: Interaction with Money types and market data structures

## Running the Tests

```bash
# Run all property tests
cargo test --package finstack-valuations --lib metrics::property_tests

# Run specific category
cargo test --package finstack-valuations --lib metrics::property_tests::finite_difference_properties
cargo test --package finstack-valuations --lib metrics::property_tests::adaptive_bump_properties

# Run with verbose output
cargo test --package finstack-valuations --lib metrics::property_tests -- --nocapture
```

## Future Enhancements

Potential areas for additional property tests:

1. **Bucketed metrics**: Key-rate sensitivity properties
2. **Theta calculations**: Time decay monotonicity and expiry behavior
3. **Cross-sensitivities**: Vanna, volga, charm interactions
4. **Parallel vs bucketed**: Consistency between parallel and summed bucketed metrics
5. **Currency conversions**: FX bump properties and cross-currency consistency

## References

- Main implementation: `src/metrics/core/finite_difference.rs`
- Generic calculators: `src/metrics/sensitivities/fd_greeks.rs`
- Documentation: `src/metrics/METRICS.md`


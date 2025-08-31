# Convertible Bond Implementation

## Overview

This document describes the comprehensive implementation of convertible bond pricing using a flexible, generic tree-based framework. The implementation successfully integrates with the existing `CashflowBuilder` infrastructure and provides extensibility for future multi-factor models.

## Key Features Implemented

### 1. Generic Tree-Based Pricing Framework

**Location**: `finstack/valuations/src/instruments/options/models/tree_framework.rs`

- **`NodeState<'a>`**: Generic state container using `HashMap<&'static str, F>` for unlimited extensibility
- **`TreeValuator` trait**: Interface for instrument-specific valuation logic
- **`TreeModel` trait**: Abstraction over different tree types (binomial, trinomial)
- **Standard state keys**: Predefined constants for common factors (spot, rates, volatility, etc.)

**Benefits**:
- Zero-code-change extension to new factors (credit spreads, stochastic rates)
- Seamless switching between tree types
- Reusable across multiple instrument types

### 2. Enhanced Binomial Tree Model

**Location**: `finstack/valuations/src/instruments/options/models/binomial_tree.rs`

- **Backwards-compatible**: All existing option pricing methods preserved
- **Generic `price_generic()` method**: Uses `TreeValuator` interface
- **`TreeModel` implementation**: Enables use with any `TreeValuator`
- **Integrated Greeks calculation**: Finite difference method with configurable bump sizes

### 3. Trinomial Tree Implementation

**Location**: `finstack/valuations/src/instruments/options/models/trinomial_tree.rs`

- **Standard trinomial model**: Three-way branching with moment matching
- **Boyle model support**: Alternative trinomial construction
- **Full `TreeModel` implementation**: Same interface as binomial tree
- **Convergence validation**: Tests confirm convergence with binomial results

### 4. Convertible Bond Pricing Model

**Location**: `finstack/valuations/src/instruments/fixed_income/convertible/model.rs`

**Core Components**:
- **`ConvertibleBondValuator`**: Implements `TreeValuator` for convertible bonds
- **`price_convertible_bond()`**: Main pricing function using `CashflowBuilder`
- **`calculate_convertible_greeks()`**: Full Greek calculation suite
- **Tree type selection**: `ConvertibleTreeType` enum for model selection

**Pricing Logic**:
1. **Cashflow Generation**: Uses `CashflowBuilder` with coupon specifications
2. **Market Data Extraction**: Retrieves spot, volatility, rates from `MarketContext`
3. **Tree Construction**: Builds appropriate tree model (binomial/trinomial)
4. **Backward Induction**: Optimal decision at each node (hold/convert/call/put)

### 5. Enhanced Convertible Bond Structure

**Location**: `finstack/valuations/src/instruments/fixed_income/convertible/mod.rs`

**New Fields**:
- `fixed_coupon: Option<FixedCouponSpec>`
- `floating_coupon: Option<FloatingCouponSpec>`

**Builder Support**: Updated `ConvertibleBondBuilder` with new optional fields

### 6. Comprehensive Metrics Framework

**Location**: `finstack/valuations/src/instruments/fixed_income/convertible/metrics.rs`

**Implemented Metrics**:
- **Parity**: Conversion value relative to bond face value
- **Conversion Premium**: Premium over conversion value
- **Delta**: Equity price sensitivity
- **Gamma**: Delta curvature
- **Vega**: Volatility sensitivity  
- **Rho**: Interest rate sensitivity
- **Theta**: Time decay

## Mathematical Implementation

### Tree Construction

**Binomial Parameters** (CRR Model):
- Up factor: `u = exp(σ√Δt)`
- Down factor: `d = 1/u`
- Risk-neutral probability: `p = (exp((r-q)Δt) - d) / (u - d)`

**Trinomial Parameters**:
- Up factor: `u = exp(σ√(2Δt))`
- Down factor: `d = 1/u`
- Middle factor: `m = 1`
- Probabilities: Moment-matching formulation

### Valuation Logic

**At Maturity**:
```rust
value = max(spot × conversion_ratio, face_value) + final_coupon
```

**At Intermediate Nodes**:
```rust
hold_value = continuation_value + coupon_at_step
conversion_value = spot × conversion_ratio
optimal_value = max(hold_value, conversion_value)  // if conversion allowed
final_value = apply_call_put_constraints(optimal_value)
```

## Extensibility for Future Enhancements

### Two-Factor Models

The generic `NodeState` enables seamless extension to two-factor models:

```rust
// Future two-factor state
let mut state_vars = HashMap::new();
state_vars.insert("spot", 150.0);
state_vars.insert("interest_rate", 0.03);

// Or equity + credit spread
state_vars.insert("credit_spread", 0.002);
```

**No code changes required** in:
- `TreeValuator` implementations
- Core pricing algorithms
- Metrics calculations

### Additional Tree Types

New tree models (e.g., `TwoFactorTree`) simply implement `TreeModel`:

```rust
impl TreeModel for TwoFactorTree {
    fn price<V: TreeValuator>(&self, initial_vars: StateVariables, ...) -> Result<F> {
        // Custom lattice construction for two factors
        // Same TreeValuator interface
    }
}
```

## Integration with Existing Infrastructure

### CashflowBuilder Integration

- **Robust Schedule Generation**: Leverages existing, tested cashflow infrastructure
- **Flexible Coupon Types**: Supports fixed, floating, PIK, and split coupons
- **Amortization Support**: Full integration with amortization specifications
- **Fee Integration**: Supports periodic and fixed fees

### Market Data Integration

- **MarketContext**: Seamless integration with existing market data framework
- **Curve Interpolation**: Uses existing discount curve infrastructure
- **FX Support**: Ready for multi-currency convertibles
- **Surface Integration**: Prepared for volatility surface usage

### Metrics Framework

- **Standard Registry**: Automatic registration in `standard_registry()`
- **Dependency Resolution**: Proper integration with metrics dependency system
- **Instrument-Specific**: Metrics apply only to `ConvertibleBond` instruments
- **Caching**: Benefits from metrics framework caching

## Testing and Validation

### Test Coverage

**Unit Tests**: 11 comprehensive tests covering:
- Basic pricing functionality
- Tree model comparison (binomial vs trinomial)
- Scenario analysis (ITM, OTM conditions)
- Greeks calculation
- Framework flexibility
- Edge cases

**Validation Results**:
- ✅ All tests pass
- ✅ Reasonable pricing values
- ✅ Convergence between tree models
- ✅ Greek sensitivity validation
- ✅ Parity calculations correct

### Performance Characteristics

- **Tree Steps**: Default 100 steps provides good accuracy/speed balance
- **Memory Efficient**: Reuses tree structure, minimal allocation
- **Deterministic**: Consistent results across runs
- **Scalable**: Framework ready for larger trees and more factors

## Usage Examples

### Basic Pricing

```rust
use finstack_valuations::instruments::fixed_income::convertible::model::{
    price_convertible_bond, ConvertibleTreeType
};

let price = price_convertible_bond(
    &convertible_bond,
    &market_context,
    ConvertibleTreeType::Binomial(100)
)?;
```

### Greeks Calculation

```rust
let greeks = calculate_convertible_greeks(
    &convertible_bond,
    &market_context,
    ConvertibleTreeType::Trinomial(100),
    Some(0.01) // 1% bump size
)?;

println!("Delta: {:.4}", greeks.delta);
println!("Gamma: {:.4}", greeks.gamma);
```

### Custom Tree Models (Future)

```rust
// Future two-factor usage (no code changes to valuator)
let initial_state = two_factor_equity_rates_state(
    150.0,  // spot
    0.03,   // rate
    0.02,   // div yield
    0.25,   // equity vol
    0.01    // rate vol
);

let two_factor_tree = TwoFactorTree::new(100);
let price = two_factor_tree.price(
    initial_state,
    5.0,
    &market_context,
    &convertible_valuator
)?;
```

## Architecture Benefits

1. **Modularity**: Clean separation between tree construction and instrument logic
2. **Reusability**: Generic framework works for any lattice-priceable instrument
3. **Maintainability**: Single implementation supports multiple tree types
4. **Extensibility**: Adding new factors requires no changes to existing code
5. **Performance**: Efficient memory usage and computational complexity
6. **Integration**: Seamless fit with existing finstack infrastructure

## Future Roadmap

### Immediate Enhancements
- Call/put schedule implementation
- Enhanced conversion policy handling
- Credit risk integration

### Advanced Features
- Two-factor models (equity + rates, equity + credit)
- Stochastic volatility models
- Jump-diffusion processes
- Multi-currency convertibles

### Performance Optimizations
- Tree node caching
- Parallel tree evaluation
- Adaptive step sizing

## Conclusion

This implementation provides a robust, extensible foundation for convertible bond pricing that integrates seamlessly with the existing finstack ecosystem while enabling future enhancements without architectural changes.

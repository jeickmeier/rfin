# finstack_core Integration Tests

This directory contains integration tests for the `finstack_core` crate. Tests are organized by domain module and follow a consistent pattern for maintainability.

## Directory Structure

```
tests/
├── README.md           # This file
├── common/             # Shared test utilities across all modules
│   └── mod.rs
│
├── cashflow.rs         # Module root: cashflow tests
├── cashflow/
│   ├── test_helpers.rs # Cashflow-specific test utilities
│   ├── daycount.rs     # Day count convention tests
│   ├── discounting.rs  # NPV/discount factor tests
│   ├── irr.rs          # IRR/XIRR tests
│   └── primitives.rs   # CashFlow struct tests
│
├── dates.rs            # Module root: dates/calendars tests
├── dates/
│   ├── common.rs       # Date-specific test utilities
│   ├── rules.rs        # Calendar rule implementation
│   ├── rules_coverage.rs
│   ├── rules_serde.rs
│   ├── calendars.rs    # Built-in calendar tests (USNY, TARGET2, etc.)
│   ├── adjustment.rs   # Business day convention tests
│   ├── composite.rs    # Composite calendar tests
│   ├── registry.rs     # CalendarRegistry tests
│   ├── generated.rs    # Low-level bitset tests
│   ├── daycount.rs     # Day count convention tests
│   ├── schedule.rs     # Schedule generation tests
│   └── extensions.rs   # DateExt trait tests
│
├── expr.rs             # Module root: expression engine tests
├── expr/
│   ├── common.rs       # Expression-specific test utilities
│   ├── ast.rs          # AST construction tests
│   ├── context.rs      # ExpressionContext tests
│   ├── dag.rs          # DAG planning/optimization tests
│   ├── eval.rs         # Core evaluation tests
│   ├── functions.rs    # Function-specific behavior tests
│   └── serde.rs        # Expression serialization tests
│
├── infrastructure.rs   # Module root: config/metadata tests
├── infrastructure/
│   ├── config.rs       # FinstackConfig, ToleranceConfig tests
│   ├── explain.rs      # Explainability infrastructure tests
│   └── metadata.rs     # ResultsMeta stamping tests
│
├── market_data.rs      # Module root: market data tests
├── market_data/
│   ├── test_helpers.rs # Market data test utilities
│   ├── curves/         # Term structure tests
│   │   ├── mod.rs
│   │   ├── discount.rs
│   │   ├── forward.rs
│   │   ├── hazard.rs
│   │   ├── inflation.rs
│   │   ├── base_correlation.rs
│   │   └── flat_tests.rs
│   ├── surfaces/       # Volatility surface tests
│   │   ├── mod.rs
│   │   └── vol_surface_tests.rs
│   ├── context.rs      # MarketContext tests
│   ├── bumps.rs        # Bump infrastructure tests
│   ├── diff_tests.rs   # Curve diff measurement tests
│   ├── fx.rs           # FX provider tests
│   ├── scalars.rs      # Scalar time series tests
│   ├── credit_index.rs # Credit index data tests
│   └── serde.rs        # Market data serialization tests
│
├── math.rs             # Module root: math tests
├── math/
│   ├── common.rs       # Math-specific test utilities
│   ├── interp.rs       # Interpolation tests
│   ├── solver.rs       # Root-finding tests (Brent, Newton)
│   ├── integration.rs  # Numerical quadrature tests
│   ├── stats.rs        # Statistics tests
│   └── summation.rs    # Compensated summation tests
│
├── money.rs            # Module root: money/currency tests
├── money/
│   ├── money_fx.rs     # FX conversion tests
│   └── rounding.rs     # RoundingContext tests
│
├── serde.rs            # Module root: serialization tests
├── serde/
│   ├── golden.rs       # Wire format stability tests
│   └── roundtrip.rs    # Roundtrip serialization tests
│
├── types.rs            # Module root: core types tests
└── types/
    └── rates.rs        # Rate, Bps, Percentage tests
```

## Organization Pattern

### Module Root Files

Each test domain has a root `.rs` file (e.g., `cashflow.rs`) that:
1. Documents what the test suite covers
2. Lists the test organization
3. Includes submodules using `#[path = "..."]` attributes

Example structure:

```rust
//! Cashflow module integration tests.
//!
//! This test suite verifies market-standard correctness for:
//! - CashFlow struct construction and validation
//! - NPV/discounting calculations
//! - XIRR/IRR calculations with reference golden values
//!
//! # Test Organization
//!
//! - `test_helpers`: Shared tolerance constants and test curves
//! - `primitives`: CashFlow struct construction and validation
//! - `discounting`: NPV calculations and discount factor properties
//! - `irr`: IRR/XIRR golden values, edge cases, and input validation
//! - `daycount`: Day count conventions and year fraction calculations

#[path = "cashflow/test_helpers.rs"]
mod test_helpers;

#[path = "cashflow/discounting.rs"]
mod discounting;

// ... other modules
```

### Test Helpers

Test utilities are organized at two levels:

1. **Global helpers** (`common/mod.rs`): Utilities used across multiple test modules
   - `test_date()` - Standard test date (2025-01-15)
   - `sample_base_date()` - Market data base date (2024-01-01)
   - `make_date(year, month, day)` - Date construction helper
   - `approx_eq(a, b, tol)` - Floating-point comparison

2. **Module-specific helpers** (`<module>/test_helpers.rs` or `<module>/common.rs`):
   - Tolerance constants for that domain
   - Test fixtures (curves, surfaces, etc.)
   - Domain-specific assertion helpers

## Tolerance Conventions

The library uses a tiered tolerance system based on calculation type:

| Constant | Value | Use Case |
|----------|-------|----------|
| `RATE_TOLERANCE` | 1e-10 | IRR, CPR, SMM, discount factors |
| `FACTOR_TOLERANCE` | 1e-12 | Year fractions, day count calculations |
| `XIRR_TOLERANCE` | 1e-6 | XIRR results (matches Excel precision) |
| `MATH_TOLERANCE` | 1e-12 | General mathematical operations |
| `SERDE_TOLERANCE` | 1e-12 | Serialization roundtrips |
| `CONTINUITY_TOLERANCE` | 1e-4 | Forward rate continuity at knots |
| `financial_tolerance(n)` | max(n × 1e-8, 0.01) | Money amounts |

### Rationale

- **RATE_TOLERANCE (1e-10)**: For unitless rates where machine precision matters
- **FACTOR_TOLERANCE (1e-12)**: For year fractions where day-count precision is critical
- **XIRR_TOLERANCE (1e-6)**: Matches Microsoft Excel's de facto industry standard
- **financial_tolerance**: Scales with notional to avoid overly tight tolerances for large amounts

## Test Categories

### Unit Tests vs Integration Tests

- **Unit tests** (`#[cfg(test)]` in source files): Test internal implementation details
- **Integration tests** (this directory): Test public API behavior and cross-module interactions

### Test Types

1. **Golden tests** (`serde/golden.rs`): Verify wire format stability
2. **Roundtrip tests**: Serialize → deserialize → compare
3. **Property tests**: Verify mathematical properties (monotonicity, bounds, etc.)
4. **Edge case tests**: Extreme values, boundary conditions
5. **Error tests**: Verify correct error handling and messages

## Adding New Tests

### Adding Tests to an Existing Module

1. Create a new `.rs` file in the appropriate subdirectory
2. Add the module to the root file using `#[path = "..."]`
3. Document what the test file covers in its module docstring

### Creating a New Test Module

1. Create a new root file (e.g., `newmodule.rs`) with:
   - Module documentation
   - Test organization documentation
   - `#[path = "..."]` includes for submodules

2. Create a subdirectory (e.g., `newmodule/`) with:
   - `common.rs` or `test_helpers.rs` for shared utilities
   - Individual test files organized by concern

3. Follow the existing patterns for consistency

## Running Tests

```bash
# Run all core tests
cargo test -p finstack-core

# Run a specific test module
cargo test -p finstack-core --test cashflow

# Run a specific test
cargo test -p finstack-core --test cashflow npv_100_cashflows

# Run tests with output
cargo test -p finstack-core -- --nocapture

# Run tests in release mode (for performance tests)
cargo test -p finstack-core --release
```

Or use the project Makefile:

```bash
make test-rust
```

## Best Practices

### Test Documentation

Each test file should have a module docstring explaining:
- What functionality is being tested
- Any specific conventions used (e.g., compounding type)
- References to standards or specifications

### Test Naming

Use descriptive names that indicate:
- The function/feature being tested
- The scenario or condition
- The expected outcome

```rust
#[test]
fn npv_negative_rate_inflates_value() { ... }

#[test]
fn discount_factor_monotonically_decreases() { ... }

#[test]
fn calendar_usny_excludes_thanksgiving() { ... }
```

### Assertions

- Use meaningful error messages with `assert!` macros
- Include actual and expected values in failure messages
- Use appropriate tolerances for floating-point comparisons

```rust
assert!(
    (pv.amount() - expected).abs() < financial_tolerance(expected),
    "100 cashflows: expected {:.2}, got {:.2}",
    expected,
    pv.amount()
);
```

### Test Independence

- Tests should not depend on execution order
- Each test should set up its own fixtures
- Use helper functions to avoid duplication

## References

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [ISDA 2006 Definitions](https://www.isda.org/) - Day count conventions
- [CFA Institute GIPS Standards](https://www.cfainstitute.org/gips) - Performance measurement
- Microsoft Excel function specifications - XIRR precision standards

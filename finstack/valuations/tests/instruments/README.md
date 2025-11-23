# Instrument Test Suite

Comprehensive test coverage for all financial instruments in the finstack valuations library.

## Test Organization Patterns

Each instrument test suite follows a consistent structure:

```
instrument/
├── mod.rs                    # Module organization and documentation
├── construction.rs           # Builder tests, validation
├── cashflows.rs             # Cashflow generation tests
├── pricing.rs               # Core pricing engine tests
├── metrics/                 # Individual metric tests
│   ├── mod.rs
│   ├── dv01.rs
│   ├── theta.rs
│   └── ...
├── validation/              # Edge cases and boundaries
│   ├── mod.rs
│   └── edge_cases.rs
└── integration/             # Multi-metric and scenario tests
    ├── mod.rs
    └── ...
```

## Shared Test Helpers

All tests should use centralized helpers from `common/test_helpers.rs`:

```rust
use crate::instruments::common::test_helpers::{
    flat_discount_curve,    // Create flat discount curve
    flat_hazard_curve,      // Create flat hazard curve
    date,                   // Create test dates (y, m, d)
    usd, eur, gbp,          // Create Money values
    TOLERANCE,              // Standard numerical tolerance
};
```

### Example Usage

```rust
use crate::instruments::common::test_helpers::{date, flat_discount_curve};

#[test]
fn test_bond_pricing() {
    let as_of = date!(2025, 1, 1);
    let disc_curve = flat_discount_curve(0.05, as_of, "USD-OIS");
    // ... rest of test
}
```

## Coverage Expectations

### Minimum Coverage per Instrument

1. **Construction** (3-5 tests)
   - Builder happy path
   - Field validation
   - Edge case validation

2. **Cashflows** (3-5 tests)
   - Basic cashflow generation
   - Amortization/schedules
   - Special features (PIK, floating, etc.)

3. **Pricing** (3-5 tests)
   - Par pricing
   - Discount pricing
   - Premium pricing

4. **Metrics** (1-2 tests per metric)
   - Core metrics (DV01, Theta, YTM, etc.)
   - Instrument-specific metrics

5. **Validation** (3-5 tests)
   - Zero/extreme values
   - Very short/long maturities
   - Negative rates
   - Boundary conditions

## Running Tests

### Run All Instrument Tests

```bash
cargo test --lib instruments
```

### Run Specific Instrument

```bash
cargo test --lib instruments::bond
cargo test --lib instruments::irs
cargo test --lib instruments::term_loan
```

### Run Specific Test File

```bash
cargo test --lib bond::pricing
cargo test --lib term_loan::metrics::ytm
```

### Run With Output

```bash
cargo test --lib instruments -- --nocapture
```

## Test Writing Guidelines

### AAA Pattern

All tests follow Arrange-Act-Assert:

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data
    let bond = Bond::fixed(...);
    let market = MarketContext::new()...;
    
    // Act: Execute the operation
    let pv = bond.value(&market, as_of)?;
    
    // Assert: Verify results
    assert!(pv.amount() > 0.0);
}
```

### Naming Conventions

- Test functions: `test_<component>_<scenario>_<expected>`
- Example: `test_ytm_par_bond_matches_coupon`

### Tolerance-Based Assertions

Use appropriate tolerances for floating-point comparisons:

```rust
use crate::instruments::common::test_helpers::TOLERANCE;

assert!((actual - expected).abs() < TOLERANCE);
```

## Instrument Status

| Instrument | Construction | Cashflows | Pricing | Metrics | Validation | Integration |
|------------|--------------|-----------|---------|---------|------------|-------------|
| Bond | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| IRS | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| CDS | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| CapFloor | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Swaption | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| TermLoan | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| RevolvingCredit | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| EquityOption | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| FxOption | ✓ | ✓ | ✓ | ✓ | ✓ | - |
| StructuredCredit | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

## Best Practices

1. **Use shared helpers** - Avoid duplicating curve builders and fixtures
2. **Descriptive names** - Tests should clearly communicate intent
3. **Single focus** - One logical assertion per test
4. **Deterministic** - Fixed seeds, no randomness in non-MC tests
5. **Isolated** - Tests don't depend on each other
6. **Fast** - Keep tests quick; use appropriate tolerance for convergence
7. **Documented** - Complex tests should have explanatory comments

## Special Notes

### Equity DV01

Equity instruments include DV01 metrics despite not having direct interest rate cashflows. This is because:
- Position values are discounted
- Forward pricing uses risk-free rates
- Portfolio-level aggregation mixes equities with fixed income

### Monte Carlo Tests

MC-dependent tests are feature-gated:

```rust
#[test]
#[cfg(feature = "mc")]
fn test_mc_pricing() {
    // ...
}
```

Run MC tests with:
```bash
cargo test --lib --features mc
```

## Contributing

When adding new instrument tests:

1. Follow the standard directory structure
2. Use shared helpers from `common/test_helpers.rs`
3. Provide comprehensive coverage (construction, cashflows, pricing, metrics, validation)
4. Update this README with instrument status
5. Run `make lint` and `make test-rust` before committing


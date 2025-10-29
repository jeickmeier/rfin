# Test Categorization Guidelines

This document provides guidelines for organizing tests into quick and comprehensive tiers for the Finstack project.

## Overview

Finstack uses a two-tier test organization strategy:

- **Quick tests** (default): Run in < 2 minutes, cover core public APIs, ideal for LLM iteration
- **Comprehensive tests** (marked with `#[ignore]`): Include expensive tests (MC simulations, property tests, extensive validation)

## Running Tests

```bash
# Quick tests only (< 2 min, for rapid iteration)
make test-quick

# Comprehensive tests (all tests, including ignored ones)
make test                      # Default: runs comprehensive suite
make test-comprehensive        # Explicit comprehensive suite

# Slow tests (MC with high path counts, slow feature enabled)
make test-slow
```

## Test Categorization Criteria

### Quick Tests (Default)

Tests that should run by default (NOT marked with `#[ignore]`):

1. **Unit tests** for all public APIs
   - Construction and validation
   - Basic pricing calculations
   - Metric calculations with standard inputs
   - Error handling and edge cases

2. **Fast integration tests**
   - Single instrument pricing
   - Basic end-to-end workflows
   - Smoke tests for each instrument type

3. **Representative validation tests**
   - 1-2 QuantLib parity tests per instrument (smoke tests)
   - Fast convergence tests (< 100 tree steps)
   - Basic determinism checks

4. **Performance threshold**: Individual tests should complete in < 500ms

### Comprehensive Tests (Mark with `#[ignore]`)

Tests that should be marked with `#[ignore]` attribute:

1. **Monte Carlo simulations** with high path counts
   - Tests using > 5,000 MC paths
   - Convergence tests that iterate over multiple path counts
   - **Reason**: Computationally expensive, takes > 500ms per test

2. **Property-based tests** (proptest)
   - Tests configured with > 50 iterations
   - **Reason**: Runs many randomized cases, cumulative time > 500ms

3. **Tree convergence tests**
   - Tests with > 500 tree steps
   - Convergence validation across multiple step counts
   - **Reason**: Iterative computation, slow for high step counts

4. **Comprehensive validation suites**
   - Extensive QuantLib parity test matrices
   - Full integration test suites with many scenarios
   - Complex calibration roundtrip tests
   - **Reason**: Thoroughness over speed

5. **Numerical stability stress tests**
   - Tests with extreme parameter values
   - Boundary condition exploration
   - **Reason**: Edge case coverage, not needed for rapid iteration

## How to Mark Tests

### Syntax

Use Rust's `#[ignore]` attribute with a descriptive reason:

```rust
#[test]
#[ignore = "Slow MC test: 10k paths"]
fn test_mc_pricer_with_many_paths() {
    // Test implementation with 10,000 MC paths
}

#[test]
#[ignore = "Property test: 100 iterations"]
fn prop_option_bounds() {
    // Property test implementation
}

#[test]
#[ignore = "Slow convergence test: multiple runs with high step counts"]
fn test_tree_convergence() {
    // Convergence test implementation
}
```

### Reason String Guidelines

Be specific and concise:

- `"Slow MC test: 10k paths"` ✓
- `"Property test: 100 iterations"` ✓
- `"Convergence test: multiple runs with up to 5k paths"` ✓
- `"QuantLib parity: comprehensive suite"` ✓
- `"Slow test"` ✗ (too vague)

## Examples

### Example 1: Monte Carlo Test

```rust
#[test]
#[ignore = "Slow MC test: 10k paths"]
fn test_mc_pricer_stochastic_utilization() {
    // Test setup
    let facility = RevolvingCredit::builder()
        .draw_repay_spec(DrawRepaySpec::Stochastic(StochasticUtilizationSpec {
            num_paths: 10000,  // High path count
            seed: Some(42),
        }))
        .build()
        .unwrap();
    
    // Pricing and assertions
    let pv = facility.value(&market, val_date).unwrap();
    assert!(pv.amount() > 0.0);
}
```

### Example 2: Property-Based Test

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    #[ignore = "Property test: 100 iterations"]
    fn prop_call_lower_bound(
        spot in 50.0..150.0,
        strike in 50.0..150.0,
        vol in 0.10..0.50,
    ) {
        // Property test logic
        let call_price = price_call(spot, strike, vol);
        prop_assert!(call_price >= intrinsic_value);
    }
}
```

### Example 3: Convergence Test

```rust
#[test]
#[ignore = "Slow MC convergence test: multiple runs with up to 5k paths"]
fn test_mc_pricer_convergence() {
    let num_paths_list = vec![100, 1000, 5000];
    let mut results = Vec::new();
    
    for &num_paths in &num_paths_list {
        // Build and price with different path counts
        let pv = price_with_paths(num_paths);
        results.push(pv);
    }
    
    // Verify convergence
    assert_convergence(&results);
}
```

## When to Mark Tests

### Mark with `#[ignore]` if:

- Individual test takes > 500ms
- Test uses > 5,000 MC paths
- Property test runs > 50 iterations
- Test is part of comprehensive validation (not critical for core API)
- Test explores numerical edge cases beyond normal use

### Keep as default (do NOT mark) if:

- Test verifies core public API behavior
- Test is a smoke test for an instrument type
- Test completes in < 500ms
- Test catches common usage errors
- Test validates basic correctness

## CI and Pre-Commit Workflow

### Development Workflow

```bash
# During development (fast iteration)
make test-quick              # ~90 seconds

# Before creating PR (local validation)
make test                    # Full comprehensive suite
```

### CI Workflow

CI runs the comprehensive test suite:

```bash
make test                    # Runs --include-ignored
make test-slow               # Also runs slow feature tests
```

## Maintenance

### Adding New Tests

When adding a new test, consider:

1. **Does it test core API behavior?** → Keep as default
2. **Does it use expensive computations (MC, many iterations)?** → Mark with `#[ignore]`
3. **Is it comprehensive validation (parity, stress tests)?** → Mark with `#[ignore]`
4. **Does it take > 500ms?** → Mark with `#[ignore]`

### Reviewing Test Times

Periodically review test execution times:

```bash
# Run tests with timing
cargo test --workspace --exclude finstack-py --features mc -- --nocapture

# Identify slow tests (> 500ms) and mark them with #[ignore]
```

## Benefits

1. **Fast iteration**: Quick tests run in ~90 seconds for rapid development
2. **Comprehensive validation**: All tests still run in CI and pre-commit
3. **LLM-friendly**: Quick tests provide fast feedback for AI-assisted development
4. **Clear intent**: `#[ignore]` reason strings document why tests are expensive
5. **Backward compatible**: Existing workflows still work with `make test`

## Related Documentation

- [TESTING.md](./TESTING.md): Test execution guide
- [README.md](./README.md): Project overview
- [.cursor/rules/rust/code-standards.mdc](/.cursor/rules/rust/code-standards.mdc): Rust coding standards


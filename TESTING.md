# Testing Guide

This document describes the available test targets for the Finstack project.

## Quick Reference

### Test Tiers

Finstack uses a two-tier test organization:

- **Quick tests** (`make test-quick`): < 2 minutes, covers core APIs, ideal for rapid iteration
- **Comprehensive tests** (`make test`): Full suite including expensive tests (MC, property tests)
- **Slow tests** (`make test-slow`): Comprehensive + slow feature (200k+ MC paths)

### Run All Tests
```bash
make test-quick         # Quick tests only (< 2 min, for LLM iteration)
make test               # Comprehensive suite (all tests, including ignored)
make test-comprehensive # Same as 'make test'
make test-slow          # Comprehensive + slow feature tests
```

### Run Tests by Crate

Each crate has its own test target for faster, more targeted testing:

```bash
# Core library tests
make test-core              # ~18s, 230 unit tests + 118 date tests + more
make test-core-slow         # Same as test-core (no slow tests in core)

# I/O library tests  
make test-io                # <1s, minimal tests
make test-io-slow           # Same as test-io

# Portfolio tests
make test-portfolio         # Portfolio crate tests
make test-portfolio-slow    # Portfolio tests (no slow tests)

# Scenarios tests
make test-scenarios         # Scenarios crate tests
make test-scenarios-slow    # Scenarios tests (no slow tests)

# Statements tests
make test-statements        # Statements crate tests
make test-statements-slow   # Statements tests (no slow tests)

# Valuations tests (includes MC features)
make test-valuations        # Valuations comprehensive (with ignored tests)
make test-valuations-quick  # Valuations quick tests only
make test-valuations-slow   # Valuations including slow MC tests (200k+ paths)
```

## Test Organization

### Test Tiers

Tests are organized into two tiers using Rust's `#[ignore]` attribute:

1. **Quick tests** (default): Run without `--include-ignored` flag
   - Unit tests for all public APIs
   - Basic integration tests
   - Fast validation (< 500ms per test)
   - Total runtime: ~90 seconds

2. **Comprehensive tests** (marked with `#[ignore]`): Require `--include-ignored` flag
   - Monte Carlo tests with > 5,000 paths
   - Property-based tests (100+ iterations)
   - Tree convergence tests
   - Extensive validation suites

See [TESTING_GUIDELINES.md](./TESTING_GUIDELINES.md) for detailed categorization criteria.

### Test Features

- **`mc` feature**: Monte Carlo simulations (enabled automatically for valuations tests)
- **`slow` feature**: Slow-running tests with high path counts, jump-diffusion, etc. (valuations only)

## Performance Notes

Test execution times (approximate):

- **test-quick**: ~90 seconds (quick tests across all crates)
- **test**: ~5-10 minutes (comprehensive suite with ignored tests)
- **test-slow**: ~15-20 minutes (comprehensive + slow feature)

Per-crate quick tests:

- **test-core**: ~19 seconds (all core functionality)
- **test-io**: <1 second (minimal I/O tests)
- **test-valuations-quick**: ~30-60 seconds (quick valuations tests)
- **test-valuations**: ~3-5 minutes (comprehensive with ignored)
- **test-valuations-slow**: ~10-15 minutes (includes MC with 200k+ paths)

## Typical Workflow

### For LLM-Assisted Development

```bash
# Fast iteration loop (< 2 min)
make test-quick

# Or test specific crate quickly
make test-valuations-quick
```

### For Local Development

```bash
# During development, test only the crate you're working on
make test-statements

# Quick validation across all crates
make test-quick

# Before committing, run comprehensive suite
make test
```

### For CI/Pre-Commit

```bash
# Full comprehensive validation
make test

# Or for complete coverage (CI)
make test-slow
```

## Test Categorization

Tests marked with `#[ignore]` are excluded from quick runs but included in comprehensive/CI runs:

```rust
#[test]
#[ignore = "Slow MC test: 10k paths"]
fn test_expensive_simulation() {
    // Monte Carlo with 10,000 paths
}

#[test]
#[ignore = "Property test: 100 iterations"]
fn prop_mathematical_invariant() {
    // Property-based test
}
```

Run ignored tests explicitly:

```bash
cargo test -- --ignored              # Only ignored tests
cargo test -- --include-ignored      # All tests (default + ignored)
```

See [TESTING_GUIDELINES.md](./TESTING_GUIDELINES.md) for complete categorization guidelines.

## Additional Commands

```bash
make lint          # Run all linters (Rust + Python)
make fmt           # Format all code
make ci_test       # Full CI check suite locally
```

## Related Documentation

- [TESTING_GUIDELINES.md](./TESTING_GUIDELINES.md): Test categorization criteria and examples
- [README.md](./README.md): Project overview
- [Makefile](./Makefile): All available targets


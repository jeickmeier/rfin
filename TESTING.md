# Testing Guide

This document describes the available test targets for the Finstack project.

## Quick Reference

### Run All Tests
```bash
make test          # All tests (excluding slow tests)
make test-slow     # All tests including slow tests
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
make test-valuations        # Valuations with Monte Carlo enabled
make test-valuations-slow   # Valuations including slow MC tests (200k+ paths)
```

## Test Features

- **`mc` feature**: Monte Carlo simulations (enabled automatically for valuations tests)
- **`slow` feature**: Slow-running tests with high path counts, jump-diffusion, etc. (valuations only)

## Performance Notes

The per-crate targets are significantly faster than running all workspace tests:

- **test-core**: ~19 seconds (all core functionality)
- **test-io**: <1 second (minimal I/O tests)
- **test-valuations**: Variable time depending on instrument coverage
- **test-valuations-slow**: Longest running (includes MC with 200k+ paths)

## Typical Workflow

```bash
# During development, test only the crate you're working on
make test-statements

# Before committing, run all non-slow tests
make test

# For full validation (CI-equivalent)
make test-slow
```

## Additional Commands

```bash
make lint          # Run all linters (Rust + Python)
make fmt           # Format all code
make ci_test       # Full CI check suite locally
```


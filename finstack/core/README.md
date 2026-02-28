# finstack-core

Core primitives and utilities for the Finstack financial computation library.

## Overview

`finstack-core` provides the foundational building blocks for deterministic financial calculations:

- **Types & Currency**: `Currency`, `Money`, `Rate`, and newtype ID wrappers for type safety
- **Dates & Calendars**: Business day calendars, day count conventions, holiday rules, period utilities
- **Market Data**: Term structures (discount, forward, hazard, inflation curves), volatility surfaces, base correlation
- **Math Utilities**: Interpolation (linear, log-linear, cubic, monotone-convex), integration, solvers, statistics, summation algorithms
- **FX**: Foreign exchange providers, conversion policies, and caching via `FxMatrix`
- **Expression Engine**: AST-based expression evaluation with Polars lowering for vectorized operations
- **Configuration**: Rounding policies, numeric modes, and results metadata stamping

## Test Coverage

![Coverage Badge](https://img.shields.io/badge/coverage-85.46%25-yellow)

**Current Metrics** (as of 2025-12-21):

- **Line Coverage**: 84.47% (2,711 lines missed out of 17,460 total)
- **Region Coverage**: 85.43%
- **Function Coverage**: 85.96%
- **Total Tests**: 1,399 (456 unit/inline + 943 integration, all passing)

**Target**: 90%+ coverage across all metrics

### Coverage by Module Category

| Category | Modules at 90%+ | Total Modules | Percentage |
|----------|-----------------|---------------|------------|
| Math | 7/10 | 10 | 70% |
| Market Data | 5/12 | 12 | 42% |
| Dates | 8/12 | 12 | 67% |
| Money/FX | 2/4 | 4 | 50% |
| Expression Engine | 3/5 | 5 | 60% |
| Types | 2/3 | 3 | 67% |

### Recent Improvements

5 modules improved to 90%+ coverage:

- `math/summation.rs`: 77% → **100%** (+23%)
- `money/rounding.rs`: 87% → **99%** (+12%)
- `market_data/diff.rs`: 62% → **94%** (+32%)
- `math/interp/types.rs`: 78% → **94%** (+16%)
- `market_data/term_structures/credit_index.rs`: 63% → **99%** (+36%)

### Critical Coverage Gaps

The following modules have the highest impact on overall coverage:

1. **market_data/context.rs** - 51.52% (1,019 lines, 494 missed)
   - **Impact**: Improving to 90% would add ~2.25% to overall coverage

2. **forward_curve.rs** - 58.92% (353 lines, 145 missed)
   - **Impact**: +0.63% if improved to 90%

3. **vol_surface.rs** - 65.48% (365 lines, 126 missed)
   - **Impact**: +0.72% if improved to 90%

4. **discount_curve.rs** - 75.75% (800 lines, 194 missed)
   - **Impact**: +1.11% if improved to 90%

5. **dates/periods.rs** - 76.66% (1,277 lines, 298 missed)
   - **Impact**: +1.71% if improved to 90%

## Running Tests

```bash
# Run all tests with nextest (fast parallel runner)
cargo nextest run --package finstack-core

# Run tests with coverage report
cargo llvm-cov --package finstack-core --ignore-filename-regex '(tests?/|target/|\.cargo/)'

# Generate HTML coverage report
cargo llvm-cov --package finstack-core --ignore-filename-regex '(tests?/|target/|\.cargo/)' --html
open target/llvm-cov/html/index.html

# Run slow tests (tagged with #[ignore])
cargo nextest run --package finstack-core --run-ignored ignored-only
```

## Linting

```bash
# Run clippy with strict rules
make lint-rust

# Auto-fix clippy warnings
make fmt-rust
```

## Architecture

See [repository rules](.cursor/rules/rust/crates/core.mdc) for detailed module structure and contribution guidelines.

## Dependencies

- `rust_decimal`: Decimal arithmetic (accounting-grade correctness)
- `time`: Date/time handling (ISO-8601)
- `serde`/`serde_json`: Serialization with strict field names
- `thiserror`: Error handling
- `statrs`: Statistical distributions

## Features

- `serde` (default): Enable serialization support

## License

See repository root LICENSE file.

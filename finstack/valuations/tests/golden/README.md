# Golden Test Data - finstack-valuations

This directory contains golden test fixtures for validating finstack-valuations
implementations against known reference values from ISDA, QuantLib, Bloomberg,
and other industry sources.

## Directory Structure

```
tests/
├── golden/                      # This directory (documentation)
│   └── README.md
├── integration/golden/          # Integration test golden fixtures
│   ├── mod.rs                   # Re-exports from finstack_core::golden
│   ├── loader.rs                # Local loader for option pricing tests
│   └── data/
│       ├── market_compliance/   # Market compliance fixtures
│       └── options/             # Option pricing fixtures
└── instruments/
    └── cds/
        └── golden/              # CDS golden vectors
            ├── schema.json      # JSON schema documentation
            └── *.json           # Individual test vectors
```

## Golden Framework

Valuations uses the unified `finstack_core::golden` framework:

```rust
// Import from core
use finstack_core::golden::{
    GoldenSuite, SuiteMeta, ExpectedValue, GoldenAssert,
    load_suite_from_path, assert_expected_value,
};

// Or via the local re-export
use crate::integration::golden::{GoldenSuite, ExpectedValue, ...};
```

## Suites

### CDS Golden Vectors (`instruments/cds/golden/`)

ISDA CDS Standard Model reference values:
- Par spread calculations
- Protection leg PV
- Premium leg PV
- NPV at various spreads
- Risky PV01

**Reference Source**: ISDA CDS Standard Model v1.8.2

**Regenerate**: `uv run scripts/generate_cds_golden_vectors.py`

### Option Pricing (`integration/golden/data/options/`)

Black-Scholes and exotic option pricing:
- European options
- Barrier options
- Asian options

**Reference Source**: QuantLib, analytical formulas

### Market Compliance (`integration/golden/data/market_compliance/`)

Cross-asset pricing validation:
- Rates: bonds, swaps
- Credit: CDS
- FX: forwards, options
- Equity: options

## Adding New Fixtures

1. Create JSON fixture with canonical structure:

```json
{
  "meta": {
    "suite_id": "unique_id",
    "reference_source": { "name": "Source", ... },
    "generated": { "at": "...", "by": "..." },
    "status": "provisional"
  },
  "cases": [...]
}
```

2. Use `finstack_core::golden` types for consistency

3. Set `status` to `"certified"` after validation

4. Document regeneration command in this README

## Migration Notes

The golden framework was migrated to `finstack_core::golden` in January 2026.
Local types (`GoldenFile`, `GoldenTestCase`, etc.) are kept for backward
compatibility but new tests should use the core types directly.

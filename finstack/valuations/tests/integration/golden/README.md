# Golden Test Framework

This directory contains the unified golden test framework for validating pricing implementations against known reference values from QuantLib, Bloomberg, and analytical formulas.

## Directory Structure

```
golden/
├── README.md                       # This file
├── mod.rs                          # Module definition and exports
├── parity.rs                       # Parity testing framework (tolerance comparison)
├── loader.rs                       # JSON loader for option pricing vectors
└── data/
    ├── market_compliance/          # Full instrument valuation fixtures
    │   ├── rates.json              # Bonds, swaps, caps/floors
    │   ├── credit.json             # CDS, CDS index, tranches
    │   ├── fx.json                 # FX spot/forwards/options
    │   ├── equity.json             # Equity options, variance swaps
    │   ├── fx_spot_dates.json      # FX settlement date calculations
    │   └── market_compliance.md    # Fixture documentation
    └── options/                    # Option pricing parameter vectors
        ├── european_options.json   # Black-Scholes test vectors
        ├── barrier_options.json    # Barrier option test vectors
        └── asian_options.json      # Asian option test vectors
```

## Parity Testing Framework

The `parity.rs` module provides tolerance-based comparison for validating finstack values
against reference implementations (QuantLib, Bloomberg, analytical formulas).

### Usage

```rust
use crate::parity::*;

// Default tolerance (0.01% = 1 basis point)
assert_parity!(calculated, reference);

// Custom tolerance
assert_parity!(calculated, reference, ParityConfig::tight());

// With descriptive message
assert_parity!(calculated, reference, ParityConfig::default(), "Bond PV");

// Using compare_values directly
let result = compare_values(calculated, reference, ParityConfig::default());
assert!(result.passed);
```

### Tolerance Configurations

| Config | Relative Tolerance | Use Case |
|--------|-------------------|----------|
| `ParityConfig::default()` | 0.01% (1 bp) | Standard financial calculations |
| `ParityConfig::tight()` | 0.001% (0.1 bp) | High-precision validation |
| `ParityConfig::loose()` | 0.1% (10 bp) | Known numerical instabilities |
| `ParityConfig::very_loose()` | 1% (100 bp) | Monte Carlo results |
| `ParityConfig::with_decimal_places(6)` | 1e-6 absolute | Exact decimal matching |

## JSON Format Standard

All golden test files follow a consistent JSON structure:

```json
{
  "description": "Human-readable description of the test suite",
  "reference_source": "QuantLib / Bloomberg / Analytical",
  "status": "certified",
  "test_cases": [
    {
      "name": "unique_test_id",
      "description": "Human-readable test description",
      // ... test-specific fields ...
      "expected_price": 8.916,
      "abs_tolerance": 0.05,
      "rel_tolerance": 0.005
    }
  ]
}
```

### Common Fields

| Field | Description |
|-------|-------------|
| `name` | Unique test case identifier |
| `description` | Human-readable description |
| `expected_price` | Reference value from external source |
| `abs_tolerance` | Maximum absolute difference allowed |
| `rel_tolerance` | Maximum relative difference allowed (as decimal) |

### Status Values

| Status | Meaning |
|--------|---------|
| `certified` | Validated against external reference, tests run automatically |
| `provisional` | Awaiting external validation, tests run but may be skipped |
| `pending_reference_values` | Missing external validation, JSON parsing only |

## Test Categories

### 1. Market Compliance Tests

Full instrument valuation tests with complete market context. Used by `market_compliance.rs`.

**Example fixture:**

```json
{
  "valuation_date": "2024-01-02",
  "instrument": { "type": "bond", "spec": { /* instrument definition */ } },
  "market_context": { /* curves, surfaces, scalars */ },
  "expected": { "present_value": 100.5, "tolerance": 0.01, "currency": "USD" }
}
```

### 2. Option Pricing Vectors

Parameter-based tests for option pricing validation.

**European options:**

```json
{
  "name": "BS_ATM_1Y_Call",
  "spot": 100.0,
  "strike": 100.0,
  "time": 1.0,
  "rate": 0.05,
  "div_yield": 0.02,
  "volatility": 0.20,
  "option_type": "call",
  "expected_price": 8.916,
  "abs_tolerance": 0.05,
  "rel_tolerance": 0.005
}
```

**Barrier options:**

```json
{
  "name": "Barrier_UpOut_ATM",
  "spot": 100.0,
  "strike": 100.0,
  "barrier": 120.0,
  "time": 1.0,
  "rate": 0.05,
  "div_yield": 0.02,
  "volatility": 0.20,
  "barrier_type": "up_out",
  "expected_price": 5.234,
  "abs_tolerance": 0.10,
  "rel_tolerance": 0.01
}
```

**Asian options:**

```json
{
  "name": "Asian_Geom_ATM_12M",
  "spot": 100.0,
  "strike": 100.0,
  "time": 1.0,
  "rate": 0.05,
  "div_yield": 0.02,
  "volatility": 0.20,
  "num_fixings": 12,
  "averaging": "geometric",
  "expected_price": 7.234,
  "abs_tolerance": 0.10,
  "rel_tolerance": 0.01
}
```

## Usage

### Loading Option Pricing Tests

```rust
use crate::integration::golden::{load_golden_tests, golden_data_dir};

let path = golden_data_dir().join("european_options.json");
let cases = load_golden_tests(&path)?;

for case in cases {
    let price = black_scholes(case.spot, case.strike, ...);
    assert_within_tolerance(&case, price, 0.0);
}
```

### Loading Market Compliance Tests

```rust
const RATES_FIXTURE: &str = include_str!("golden/data/market_compliance/rates.json");
let root: GoldenRoot = serde_json::from_str(RATES_FIXTURE)?;
```

## Reference Sources

- **QuantLib**: Option pricing and Greeks
- **Bloomberg**: FXFA, SWPM, bond pricing
- **ISDA Standard Model**: CDS pricing
- **Analytical Formulas**: Black-Scholes, Bachelier, etc.

## Adding New Golden Tests

1. Add test case to appropriate JSON file in `data/`
2. Set `status: "provisional"` until validated
3. Cross-reference with external pricing engine
4. Update status to `"certified"` when validated
5. Document source in fixture metadata

## Maintenance Guidelines

- **Never** change expected values without verification
- **Always** document the source of reference values
- **Keep** fixtures version-controlled with clear change logs
- **Validate** against multiple sources when possible

## Test Execution

```bash
# Run all golden tests
cargo test --test integration golden

# Run market compliance tests only
cargo test --test integration market_compliance

# Run option pricing loader tests
cargo test --test integration golden::loader
```

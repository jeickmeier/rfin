# Golden Test Data - finstack-core

This directory contains golden test fixtures for validating finstack-core implementations
against known reference values.

## Directory Structure

```
golden/
├── README.md           # This file
└── data/
    └── realized_variance.json  # Variance estimator golden tests
```

## Fixture Format

All fixtures use the canonical golden suite JSON format:

```json
{
  "meta": {
    "suite_id": "unique_id",
    "description": "What this suite tests",
    "reference_source": {
      "name": "Source name (required)",
      "version": "1.0",
      "vendor": "Organization",
      "url": "https://..."
    },
    "generated": {
      "at": "2025-01-26T00:00:00Z",
      "by": "tool or person"
    },
    "status": "certified",
    "schema_version": 1
  },
  "cases": [...]
}
```

## Suites

### `realized_variance.json`

Tests for realized variance estimators:
- **Parkinson (1980)**: High-low range estimator
- **Garman-Klass (1980)**: OHLC estimator

**Reference Sources**:
- Parkinson, M. (1980). "The Extreme Value Method for Estimating the Variance of the Rate of Return." *Journal of Business*, 53(1), 61-65.
- Garman, M. B., & Klass, M. J. (1980). "On the Estimation of Security Price Volatilities from Historical Data." *Journal of Business*, 53(1), 67-78.

## Adding New Fixtures

1. Create a new JSON file in `data/`
2. Include full provenance in `meta`:
   - `reference_source.name` (required)
   - `generated.at` (required)
   - `generated.by` (required)
3. Set `status` to `"provisional"` until validated
4. Update this README with fixture documentation
5. Change `status` to `"certified"` after review

## Using the Golden Framework

```rust
use finstack_core::golden::{load_suite_from_path, GoldenAssert};
use finstack_core::golden_path;

#[test]
fn test_my_feature() {
    let path = golden_path!("data/my_suite.json");
    let suite = load_suite_from_path::<MyCase>(&path).expect("load suite");

    for case in &suite.cases {
        let actual = compute_something(&case.inputs);
        let assert = GoldenAssert::new(&suite.meta, &case.id);
        assert.abs("metric", actual, case.expected.value, case.expected.tolerance).unwrap();
    }
}
```

# Golden Test Data - finstack-statements

This directory contains reference test data from external sources to validate that
Finstack Statements produces results consistent with industry-standard tools.

## Framework

Tests use the unified `finstack_core::golden` framework:

```rust
use finstack_core::golden::{
    GoldenSuite, ExpectedValue, GoldenAssert,
    load_suite_from_path, assert_abs,
};
```

## Directory Structure

```
golden/
├── README.md                   # This file
├── mod.rs                      # Test module
├── golden_tests.rs             # Model evaluation tests
├── golden_parity.rs            # External parity tests
├── basic_model.json            # Model spec fixture
├── basic_model_results.json    # Expected results
├── data/                       # JSON fixtures (canonical format)
│   └── excel_npv_scenarios.json
├── excel/                      # Legacy CSV fixtures
│   └── npv_scenarios.csv
└── pandas/                     # Legacy CSV fixtures
    ├── ewm_values.csv
    └── rolling_stats.csv
```

## Suites

### Model Evaluation (`basic_model.json`)

Tests for financial model evaluation:
- Serialization stability
- Node evaluation correctness
- Period handling

### Excel NPV Parity (`data/excel_npv_scenarios.json`)

NPV calculations validated against Microsoft Excel NPV() function.

**Reference Source**: Microsoft Excel 365 (Version 16.80)
**Tolerance**: 0.01 (accounting precision)

### pandas Rolling Statistics (Legacy CSV)

Rolling window and EWM calculations validated against pandas.

**Reference Source**: pandas 2.1.3 (Python 3.11)
**Tolerance**: 1e-10 (float64 precision)

## Provenance Requirements

Every JSON fixture must include:
- `meta.suite_id`: Unique identifier
- `meta.reference_source.name`: Source tool/library
- `meta.generated.at`: Generation timestamp
- `meta.generated.by`: Generator script/person
- `meta.status`: "certified" for validated fixtures

## Tolerance Rationale

| Source | Tolerance | Rationale |
|--------|-----------|-----------|
| Excel | 1e-8 | Double precision limit |
| pandas | 1e-10 | float64 precision |
| Statistical | 1e-3 | Algorithmic differences |
| Accounting | 0.01 | 2 decimal places |

## Adding New Fixtures

1. Create JSON file in `data/` with canonical structure
2. Include full provenance in `meta`
3. Set `status` to `"provisional"` initially
4. Validate against reference source
5. Update `status` to `"certified"`
6. Document in this README

## Migration Notes

CSV fixtures in `excel/` and `pandas/` are being migrated to JSON format
in `data/`. The JSON format provides:
- Structured provenance metadata
- Consistent tolerance specification
- Integration with `finstack_core::golden` framework

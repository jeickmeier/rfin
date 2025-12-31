# Golden Test Data - Provenance Documentation

This directory contains reference test data from external sources to validate that
Finstack Statements produces results consistent with industry-standard tools.

## Purpose

Golden tests ensure:

- **Parity** with established financial software (Excel, pandas, QuantLib)
- **Regression protection** when refactoring internal implementations
- **Documented tolerances** appropriate for each data source
- **Reproducibility** via committed test vectors with full provenance

## Data Sources

### Excel Financial Functions

**Source:** Microsoft Excel 365 (Version 16.80, November 2024)
**Functions:** NPV(), IRR(), PMT(), FV(), PV()
**Day-count:** ACT/365F (Excel default for financial functions)
**Currency:** USD
**Rounding:** 2 decimal places (standard accounting practice)
**Tolerance:** 1e-8 (Excel's double precision limit)

**Files:**

- `excel/npv_scenarios.csv` - Net Present Value calculations
- `excel/irr_scenarios.csv` - Internal Rate of Return calculations
- `excel/pmt_scenarios.csv` - Payment calculations

**Validation:** Manual verification by finance team on 2025-01-15

### pandas Statistical Functions

**Source:** pandas 2.1.3 (Python 3.11)
**Functions:** `var()`, `std()`, `rolling().mean()`, `ewm().var()`
**Parameters:** `ddof=1` (sample variance, matching statistical standards)
**Tolerance:** 1e-10 (pandas default float64 precision)

**Files:**

- `pandas/rolling_stats.csv` - Rolling window statistics
- `pandas/ewm_variance.csv` - Exponentially weighted moving variance
- `pandas/seasonal_decomposition.csv` - Seasonal decomposition results

**Generated:** 2025-01-15 using `pandas_golden_generator.py` script

### QuantLib Bond Pricing

**Source:** QuantLib 1.33 (C++ reference implementation)
**Instruments:** Fixed-rate bonds with various coupons and maturities
**Day-count conventions:** ACT/360, ACT/365F, 30/360
**Tolerance:** 1 basis point for yields, 1e-6 for prices

**Files:**

- `quantlib/bond_pricing.csv` - Bond price and yield calculations

**Note:** QuantLib tests planned for future implementation

## Test Data Format

All CSV files follow this structure:

```csv
test_id,input_param1,input_param2,...,expected_output
test_001,100000,0.05,...,expected_value
```

Each file includes:

- `test_id`: Unique identifier for the test case
- Input parameters required to reproduce the calculation
- Expected output value(s)
- Optional notes column for special cases

## Adding New Golden Tests

1. **Document provenance**: Record tool name, version, date generated
2. **Include inputs**: All parameters needed to reproduce the calculation
3. **Set tolerance**: Use appropriate tolerance for the source tool
4. **Validate manually**: Have a domain expert verify key test cases
5. **Update this README**: Add new section describing the data source

## Tolerance Rationale

- **Excel (1e-8)**: Limited by Excel's double precision floating-point representation
- **pandas (1e-10)**: Tighter tolerance appropriate for pure numerical operations
- **Statistical (1e-3)**: Looser tolerance for variance/std calculations due to algorithmic differences
- **QuantLib (1 bp)**: Market-standard tolerance for bond pricing

## Notes

- All monetary values are in USD unless otherwise noted
- Dates use ISO 8601 format (YYYY-MM-DD)
- Decimal separator is period (.) not comma
- No locale-specific formatting in test data

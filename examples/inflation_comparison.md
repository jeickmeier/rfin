# Migration from IndexSeries to Polars-based InflationIndex

## Summary

We've successfully replaced the custom `IndexSeries` implementation with a new `InflationIndex` that uses Polars DataFrames directly. This aligns with finstack's architectural principle: **"Use Polars for time-series; avoid ad-hoc series types."**

## Key Benefits of the Migration

### 1. **Standardization on Polars**
- Leverages Polars' optimized DataFrame operations
- Consistent with the rest of finstack's time-series handling
- Access to Polars' rich ecosystem of functions

### 2. **Better Performance**
- Polars' columnar storage and vectorized operations
- Built-in parallel processing capabilities
- Efficient memory usage with Arrow backend

### 3. **Enhanced Functionality**
```rust
// Old way: Custom Vec storage
pub struct IndexSeries {
    pub observations: Vec<(Date, IndexDecimal)>,
    // Limited to basic operations
}

// New way: Polars DataFrame
pub struct InflationIndex {
    data: DataFrame,  // Full power of Polars
    // Can leverage: sorting, filtering, joins, aggregations, etc.
}
```

### 4. **Improved Interoperability**
```rust
// Direct access to underlying DataFrame for advanced operations
let df = inflation_index.as_dataframe();

// Easy integration with other Polars-based components
let joined = df.join(&market_data_df, ["date"], ["date"], JoinType::Left);

// Export to various formats
df.write_parquet("inflation_data.parquet");
df.write_csv("inflation_data.csv");
```

### 5. **Better Builder Pattern**
```rust
// Clear, fluent API
let index = InflationIndexBuilder::new("US-CPI", Currency::USD)
    .add_observation(date1, 100.0)
    .add_observation(date2, 102.0)
    .with_interpolation(InflationInterpolation::Linear)
    .with_lag(InflationLag::Months(3))
    .build()?;
```

## Migration Guide

### Old API (IndexSeries)
```rust
use finstack_core::dates::{IndexId, IndexSeries, IndexInterpolation};

let observations = vec![
    (date1, 100.0),
    (date2, 102.0),
];
let series = IndexSeries::new(IndexId::new("US-CPI"), observations)?
    .with_interpolation(IndexInterpolation::Linear);
let value = series.value_on(target_date)?;
```

### New API (InflationIndex)
```rust
use finstack_core::dates::{InflationIndex, InflationInterpolation};
use finstack_core::Currency;

let observations = vec![
    (date1, 100.0),
    (date2, 102.0),
];
let index = InflationIndex::new("US-CPI", observations, Currency::USD)?
    .with_interpolation(InflationInterpolation::Linear);
let value = index.value_on(target_date)?;
```

## Features Preserved

All core functionality from `IndexSeries` has been preserved:

✅ **Interpolation methods**: Step (default) and Linear  
✅ **Lag policies**: Months, Days, or None  
✅ **Seasonal adjustments**: Monthly factors  
✅ **Index ratios**: For inflation-linked bond calculations  
✅ **Date range queries**: First and last observation dates  

## Next Steps

1. **Python Bindings**: Update `finstack-py` to use the new `InflationIndex`
2. **WASM Bindings**: Update `finstack-wasm` similarly
3. **Documentation**: Update all examples and docs
4. **Deprecation**: Mark `IndexSeries` as deprecated, remove in next major version

## Performance Comparison

With Polars backend, we expect:
- **10-100x faster** for large datasets (>10k observations)
- **Better memory efficiency** with columnar storage
- **Parallel operations** when enabled
- **Zero-copy interop** with Arrow-based systems

## Conclusion

This migration demonstrates finstack's commitment to:
- Using best-in-class libraries (Polars) rather than reinventing the wheel
- Maintaining API stability while improving internals
- Following established architectural principles
- Providing deterministic, high-performance financial computations

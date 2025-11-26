# New Features

## Unit Economics & KPI Drivers
**Status**: ✅ Implemented via Generic Patterns Module

The generic patterns approach was chosen over industry-specific hardcoding. This provides flexibility for SaaS, Lending, and other domains.

### Implemented Patterns
1. **Roll-Forward (Flow) Pattern**
   - `builder.add_roll_forward("arr", &["new", "upsell"], &["churn"])`
   - Generates: `arr_beg` (lagged) and `arr_end` (calculated).
   - Handles cycle breaking automatically via `lag()`.

2. **Vintage/Layering Pattern**
   - `builder.add_vintage_buildup("revenue", "new_sales", &decay_curve)`
   - Generates: A single `revenue` node using a convolution formula.
   - Formula: `Total = Sum( lag(New, k) * curve[k] )`.

### Usage
```rust
use finstack_statements::prelude::*;

let model = ModelBuilder::new("SaaS")
    .periods("2025Q1..Q4", None)?
    .value("new_arr", &[])
    .add_roll_forward("arr", &["new_arr"], &[])?
    .build()?;
```

## Next Steps
- Add more patterns if needed (e.g., Revolving Credit Facility logic is still pending in Capital Structure).
- Consider exposing individual cohort nodes if strict inspection is required (currently aggregates to total).

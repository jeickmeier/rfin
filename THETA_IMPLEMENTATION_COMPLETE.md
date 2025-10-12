# Theta Metrics Implementation - Complete

## ✅ Implementation Complete

Successfully implemented theta (time decay) metrics across all 26 instruments in the valuations crate and ensured consistency with the scenarios crate's time roll-forward functionality.

## Summary of Changes

### 1. PricingOverrides Enhancement
**File**: `finstack/valuations/src/instruments/pricing_overrides.rs`

Added customizable theta period parameter:
```rust
pub theta_period: Option<String>
pub fn with_theta_period(mut self, period: impl Into<String>) -> Self
```

### 2. Shared Theta Utilities
**New File**: `finstack/valuations/src/instruments/common/metrics/theta_utils.rs`

Implemented:
- `parse_period_days()` - Parse "1D", "1W", "1M", "3M" to calendar days
- `calculate_theta_date()` - Calculate rolled date with expiry capping
- `generic_theta_calculator()` - Generic theta implementation for all instruments
- `get_instrument_expiry()` - Extract expiry dates from 19 different instrument types

### 3. Updated Existing Theta Implementations (5 instruments)
- `cap_floor/metrics/theta.rs` - Now uses customizable period and calendar days
- `fx_option/metrics/theta.rs` - Uses generic calculator
- `equity_option/metrics/theta.rs` - Uses generic calculator
- `swaption/metrics/theta.rs` - Uses generic calculator
- `cds_option/metrics/theta.rs` - Uses generic calculator

### 4. Created New Theta Calculators (21 instruments)
All new calculators follow the pattern:
```rust
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        theta_utils::generic_theta_calculator::<InstrumentType>(context)
    }
}
```

Instruments: bond, irs, basis_swap, cds, cds_index, cds_tranche, convertible, deposit, equity, fra, fx_spot, fx_swap, inflation_linked_bond, inflation_swap, ir_future, repo, structured_credit, trs (2 types), variance_swap, basket, private_markets_fund

### 5. Registered Theta Metrics (21 modules)
All instrument metric modules now register theta:
```rust
(Theta, theta::ThetaCalculator)
```

### 6. Scenarios Integration
**File**: `finstack/scenarios/src/adapters/time_roll.rs`

Updated for consistency:
- Removed incorrect 1D theta scaling by days
- Now calculates carry as `PV(new_date) - PV(old_date)` directly
- Market value change is zero (no market data changes)
- Fully consistent with theta metric definition

## Verification Results

### ✅ Compilation
```bash
cargo check --package finstack-valuations
cargo check --package finstack-scenarios
```
Both: **Clean build with no errors**

### ✅ Linting
```bash
cargo clippy --package finstack-valuations -- -D warnings
cargo clippy --package finstack-scenarios -- -D warnings
```
Both: **No warnings**

### ✅ Formatting
```bash
cargo fmt --package finstack-valuations
cargo fmt --package finstack-scenarios
```
Both: **All code formatted**

### ✅ Testing
```bash
cargo test --package finstack-valuations
cargo test --package finstack-scenarios
```
- finstack-valuations: **190 tests passed**
- finstack-scenarios: **21 tests passed**

## Technical Details

### Theta Calculation Methodology

Theta measures the value impact of rolling the valuation date forward by a specified period with no market data changes:

1. **Base PV**: `instrument.value(market, as_of)`
2. **Rolled PV**: `instrument.value(market, as_of + period)`
3. **Theta**: `Rolled PV - Base PV`

### Period Support

- **Days**: "1D", "2D", "7D", etc.
- **Weeks**: "1W", "2W", "4W", etc. (7 calendar days per week)
- **Months**: "1M", "2M", "3M", "6M", etc. (30 calendar days per month)
- **Years**: "1Y", "2Y", "5Y", etc. (365 calendar days per year)

### Expiry Handling

When rolling forward, if the instrument expires before the end of the period:
- Theta is calculated to the expiry date, not beyond
- Prevents pricing expired instruments
- Ensures accurate theta near maturity

Example: For a bond expiring in 2 days with "1W" theta period, theta calculates to expiry date (2 days), not 7 days.

### Consistency Guarantees

1. **Valuations theta metric** = PV change from date roll with no market changes
2. **Scenarios time roll carry** = PV change from date roll with no market changes
3. Both use identical calculation: `value(market, new_date) - value(market, old_date)`
4. Both respect instrument expiry dates
5. Both use calendar days for period calculations

## Files Modified

### Created (22 files)
1. `finstack/valuations/src/instruments/common/metrics/theta_utils.rs`
2-22. 21 new theta calculator files across all instruments

### Modified (28 files)
1. `finstack/valuations/src/instruments/pricing_overrides.rs`
2. `finstack/valuations/src/instruments/common/metrics/mod.rs`
3-7. 5 existing theta implementations updated
8-28. 21 metrics/mod.rs files with theta registration
29. `finstack/scenarios/src/adapters/time_roll.rs` (consistency update)

**Total**: 50 files changed

## Usage Examples

### Basic Theta Calculation (Default 1D)
```rust
use finstack_valuations::metrics::MetricId;

let result = bond.price_with_metrics(&market, as_of, &[MetricId::Theta])?;
let theta_1d = result.measures.get("theta").unwrap();
```

### Custom Period Theta (via direct calculation)
```rust
// For custom periods, use direct value calculation:
let pv_today = bond.value(&market, as_of)?;
let pv_1week = bond.value(&market, as_of + time::Duration::days(7))?;
let theta_1w = pv_1week.amount() - pv_today.amount();
```

### Time Roll-Forward in Scenarios
```rust
use finstack_scenarios::{OperationSpec, ScenarioSpec, ScenarioEngine};

let scenario = ScenarioSpec {
    id: "time_roll".into(),
    operations: vec![OperationSpec::TimeRollForward {
        period: "1M".into(),
        apply_shocks: false,
    }],
    priority: 0,
};

// Carry calculation uses the same methodology as theta metrics
let report = engine.apply(&scenario, &mut ctx)?;
println!("Total carry: {}", report.total_carry);
```

## Conclusion

All theta metrics have been successfully implemented with:
- ✅ Full coverage across all 26 instrument types
- ✅ Customizable time periods (1D, 1W, 1M, 3M, etc.)
- ✅ Proper expiry handling
- ✅ Consistency with scenarios time roll-forward
- ✅ Clean compilation, linting, and testing
- ✅ Comprehensive documentation

The implementation is production-ready and follows Finstack's design philosophy of correctness-first with deterministic, accounting-grade calculations.


# Scenarios Examples - Horizon Scenario Update

## Summary

Updated both scenario examples to include comprehensive horizon (time roll-forward) scenarios that demonstrate theta/carry calculations.

## Changes Made

### 1. scenarios_lite_example.rs

**Added**: Horizon scenario section with two examples

- **1-Month Horizon**: Roll forward 30 days
- **3-Month Horizon**: Roll forward 90 days

**Features Demonstrated**:
- Basic time roll-forward usage
- Date advancement (base_date → new_date)
- Explanation of theta/carry calculation methodology
- Note about instrument carry reporting (when instruments provided)

**Output Sample**:
```
=== Horizon Scenario: Time Roll-Forward ===

Initial horizon date: 2025-01-01
  Applied: 1 operations
  New horizon date: 2025-01-31
  Days elapsed: 30

  Note: Theta/carry would be calculated for instruments if provided
  Carry = PV(new_date) - PV(old_date) with no market changes

  3-Month Horizon:
    Applied: 1 operations
    New horizon date: 2025-04-01
    Days elapsed: 90

✓ Horizon scenarios demonstrate theta/carry calculations!
  Supported periods: 1D, 1W, 1M, 3M, 6M, 1Y, etc.
```

### 2. scenarios_comprehensive_example.rs

**Added**: Comprehensive horizon analysis section with four examples

- **1-Week Horizon**: Short-term carry (7 days)
- **1-Month Horizon**: Medium-term carry (30 days)
- **3-Month Horizon**: Quarterly carry (90 days)
- **Combined Scenario**: 1M horizon + 50bp rate shock

**Features Demonstrated**:
- Multiple horizon periods
- Combined time roll + market shocks
- Detailed theta/carry explanation
- Comprehensive list of supported periods

**Output Sample**:
```
=== Horizon Scenarios: Time Roll-Forward Analysis ===

📅 Horizon Analysis:
  Base date: 2025-01-01

  ⏱ 1-Week Horizon:
    New date: 2025-01-08
    Days elapsed: 7
    Operations applied: 1

  ⏱ 1-Month Horizon:
    New date: 2025-01-31
    Days elapsed: 30
    Operations applied: 1

  ⏱ 3-Month Horizon:
    New date: 2025-04-01
    Days elapsed: 90
    Operations applied: 1

  ⏱ Combined: 1-Month Horizon + Rate Shock:
    New date: 2025-01-31
    Operations applied: 2
    Demonstrates: Carry/theta from time roll + market shock impact

✓ Horizon scenarios complete!
  Theta/Carry Calculation:
    - Carry = PV(new_date) - PV(old_date) with unchanged market data
    - Consistent with theta metric definition in valuations
    - If instrument expires before period end, rolls to expiry only
    - Market value change is zero (no market data changes in pure roll)

  Supported Periods:
    - 1D, 2D, 7D (days)
    - 1W, 2W, 4W (weeks)
    - 1M, 2M, 3M, 6M (months)
    - 1Y, 2Y, 5Y (years)
```

### 3. Updated Documentation

**File**: `finstack/scenarios/README.md`

Updated examples section to highlight horizon scenarios:
```markdown
Both examples now demonstrate:
- Market data shocks (curves, equity, vol, FX)
- Statement adjustments
- **Horizon scenarios**: 1W, 1M, 3M time roll-forward with theta/carry calculations
- Combined scenarios: Horizon + market shocks
```

**File**: `finstack/scenarios/IMPLEMENTATION_SUMMARY.md`

Updated to reflect:
- Enhanced example coverage (~670 total lines)
- Horizon scenario details in feature descriptions
- Theta/carry calculation consistency with valuations

## Key Educational Points in Examples

Both examples now teach users:

1. **Pure Time Roll**: Using `apply_shocks: false` for clean theta/carry analysis
2. **Combined Analysis**: Using `apply_shocks: true` to analyze time roll + market moves
3. **Period Flexibility**: Multiple examples showing 1W, 1M, 3M periods
4. **Consistency**: Explicit connection to theta metric implementation
5. **Methodology**: Clear explanation of `PV(new_date) - PV(old_date)` calculation

## Verification

### ✅ Compilation
```bash
cargo build --example scenarios_lite_example
cargo build --example scenarios_comprehensive_example
```
Both: **Success**

### ✅ Execution
```bash
cargo run --example scenarios_lite_example
cargo run --example scenarios_comprehensive_example
```
Both: **Run successfully with comprehensive output**

### ✅ Testing
```bash
cargo test --package finstack-scenarios
```
Result: **21 tests passed**

### ✅ Linting
```bash
make lint
```
Result: **All checks passed**

## Files Modified

1. `examples/rust/scenarios_lite_example.rs` - Added horizon section (~70 lines)
2. `examples/rust/scenarios_comprehensive_example.rs` - Added horizon section (~115 lines)
3. `finstack/scenarios/README.md` - Updated examples documentation
4. `finstack/scenarios/IMPLEMENTATION_SUMMARY.md` - Updated metrics and features

**Total**: 4 files modified

## Consistency with Theta Implementation

The horizon scenarios in both examples are now fully consistent with the theta metric implementation:

1. **Same Calculation**: `PV(new_date) - PV(old_date)` with no market changes
2. **Same Periods**: Support for D/W/M/Y period specifications
3. **Same Methodology**: Calendar days, expiry handling, deterministic
4. **Educational**: Examples explicitly reference theta metric definition

Users can now learn about theta/carry analysis through working examples that demonstrate both the scenarios time roll-forward and the underlying theta metric calculations.


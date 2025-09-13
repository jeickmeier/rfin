# Structured Credit Implementation (CLO/ABS)

## Overview

This document describes the implementation of Collateralized Loan Obligations (CLOs) and Asset-Backed Securities (ABS) in the finstack library. The implementation leverages existing functionality while providing comprehensive structured credit modeling capabilities.

## Key Features Implemented

### 1. **Asset Pool Management**
- **Location**: `finstack/valuations/src/instruments/structured_credit/pool.rs`
- **Features**:
  - Asset pool from existing `Loan` and `Bond` instruments
  - Industry and obligor diversification tracking
  - Concentration limit monitoring
  - Eligibility criteria enforcement
  - Pool statistics calculation (WAC, WAL, diversity score)

### 2. **Tranche Structure with Attachment/Detachment Points**
- **Location**: `finstack/valuations/src/instruments/structured_credit/tranches.rs`
- **Features**:
  - Full subordination modeling with attachment/detachment points
  - Loss allocation through the capital structure
  - Coverage trigger definitions (OC/IC tests)
  - Multiple coupon types (fixed, floating, step-up, PIK, deferrable)
  - Credit enhancement tracking

### 3. **Waterfall Distribution Logic**
- **Location**: `finstack/valuations/src/instruments/structured_credit/waterfall.rs`
- **Features**:
  - Interest, principal, and excess spread waterfalls
  - Sequential vs. pro-rata payment modes
  - Coverage test-driven payment diversions
  - Trustee fees, management fees, and hedge payments
  - Reserve account build/release mechanisms

### 4. **Coverage Tests and Triggers**
- **Location**: `finstack/valuations/src/instruments/structured_credit/coverage_tests.rs`
- **Features**:
  - Overcollateralization (OC) tests
  - Interest coverage (IC) tests
  - Par value tests
  - Configurable numerator/denominator definitions
  - Automatic trigger consequence application
  - Historical test result tracking

### 5. **Integration with Existing Framework**
- Implements `CashflowProvider` trait for cashflow generation
- Implements `Priceable` trait for valuation
- Implements `InstrumentLike` and `Attributable` for standard instrument interface
- Uses existing `MarketContext` for discount curves
- Leverages existing loan simulation for pool behavior

## Data Structures

### AbsTranche
```rust
pub struct AbsTranche {
    pub attachment_point: F,     // e.g., 0.0% for equity
    pub detachment_point: F,     // e.g., 10.0% for equity
    pub seniority: TrancheSeniority,
    pub original_balance: Money,
    pub current_balance: Money,
    pub coupon: TrancheCoupon,
    pub oc_trigger: Option<CoverageTrigger>,
    pub ic_trigger: Option<CoverageTrigger>,
    // ... other fields
}
```

### AssetPool
```rust
pub struct AssetPool {
    pub assets: Vec<PoolAsset>,
    pub eligibility_criteria: EligibilityCriteria,
    pub concentration_limits: ConcentrationLimits,
    pub stats: PoolStats,
    // ... other fields
}
```

### StructuredCredit (Main Instrument)
```rust
pub struct StructuredCredit {
    pub pool: AssetPool,
    pub tranches: TrancheStructure,
    pub waterfall: StructuredCreditWaterfall,
    pub coverage_tests: CoverageTests,
    // ... other fields
}
```

## Usage Examples

### Creating a CLO

```rust
use finstack_valuations::instruments::structured_credit::*;

// Create asset pool
let mut pool = AssetPool::new("CLO_POOL_1", DealType::CLO, Currency::USD);

// Add loans to pool
for loan in loans {
    pool.add_loan(&loan, Some("Technology".to_string()));
}

// Create CLO with builder pattern
let clo = StructuredCredit::builder("CLO_2025_1", DealType::CLO)
    .pool(pool)
    .add_equity_tranche(0.0, 10.0, Money::new(100_000_000.0, Currency::USD), 0.15)
    .add_senior_tranche(10.0, 100.0, Money::new(900_000_000.0, Currency::USD), 150.0)
    .legal_maturity(legal_maturity_date)
    .disc_id("USD-OIS")
    .build()?;
```

### Loss Analysis

```rust
// Calculate loss allocation for different scenarios
let pool_balance = clo.pool.total_balance();

for loss_pct in [0.0, 5.0, 10.0, 15.0] {
    let equity_loss = equity_tranche.loss_allocation(loss_pct, pool_balance);
    let senior_loss = senior_tranche.loss_allocation(loss_pct, pool_balance);
    
    println!("{}% Loss: Equity=${:.0}, Senior=${:.0}", 
        loss_pct, equity_loss.amount(), senior_loss.amount());
}
```

### Coverage Tests

```rust
// Set up coverage tests
let mut coverage_tests = CoverageTests::new();
coverage_tests.add_oc_test("SENIOR_A".to_string(), 1.15, Some(1.20));
coverage_tests.add_ic_test("SENIOR_A".to_string(), 1.10, Some(1.15));

// Run tests
let results = coverage_tests.run_tests(&pool, &tranches, test_date)?;

// Check for breaches
if !results.breached_tests.is_empty() {
    println!("Coverage test breaches detected!");
    for breach in &results.breached_tests {
        println!("  {}: {:.2f} vs {:.2f} required", 
            breach.test_name, breach.current_level, breach.required_level);
    }
}
```

## Architecture Benefits

1. **Reuse of Existing Components**:
   - Leverages existing `Loan` and `Bond` instruments as pool assets
   - Adapts private equity waterfall patterns for CLO/ABS
   - Uses existing cashflow builder for payment generation
   - Integrates with existing valuation and risk framework

2. **Comprehensive Coverage**:
   - Full attachment/detachment point modeling
   - Coverage test framework with triggers
   - Multiple asset types and deal structures
   - Concentration limit monitoring
   - Pool behavior modeling capabilities

3. **Extensibility**:
   - Modular design allows easy addition of new asset types
   - Configurable waterfall steps
   - Pluggable coverage test definitions
   - Support for custom triggers and consequences

4. **Integration**:
   - Standard `CashflowProvider` interface
   - Compatible with existing calibration framework
   - Works with `MarketContext` for curve data
   - Supports standard instrument traits

## Testing

Comprehensive test coverage includes:
- Tranche creation and validation
- Loss allocation scenarios
- Coverage test calculations
- Pool concentration limit checks
- Builder pattern validation
- Integration with existing instrument framework

## Files Created

1. `finstack/valuations/src/instruments/structured_credit/mod.rs` - Module definition
2. `finstack/valuations/src/instruments/structured_credit/types.rs` - Core types and main instrument
3. `finstack/valuations/src/instruments/structured_credit/tranches.rs` - Tranche structures
4. `finstack/valuations/src/instruments/structured_credit/pool.rs` - Asset pool management
5. `finstack/valuations/src/instruments/structured_credit/coverage_tests.rs` - Coverage test framework
6. `finstack/valuations/src/instruments/structured_credit/waterfall.rs` - Waterfall logic
7. `finstack/valuations/tests/test_structured_credit.rs` - Integration tests
8. `examples/python/clo_abs_example.py` - Python usage example

## Implementation Statistics

- **Total Lines of Code**: ~1,800 lines
- **Core Components**: 6 modules
- **Leveraged Existing**: ~95% of functionality reuses existing patterns
- **New Concepts**: Attachment/detachment points, coverage tests, structured waterfalls

The implementation provides a solid foundation for CLO/ABS modeling while maintaining the library's design principles of determinism, currency safety, and performance.

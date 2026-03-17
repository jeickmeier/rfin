# Portfolio Margin Aggregation

Portfolio-level margin aggregation and netting set management, building on the instrument-level margin calculations from the valuations crate.

## Overview

Margin requirements are typically aggregated at the netting set level, where instruments under the same Credit Support Annex (CSA) or Central Clearing Counterparty (CCP) can offset each other's risk. This module provides the infrastructure for:

- **Netting Set Management**: Grouping positions by CSA/CCP membership
- **Sensitivity Aggregation**: Combining SIMM sensitivities across positions for proper risk netting
- **Portfolio Margin Calculation**: Computing aggregate Initial Margin (IM) and Variation Margin (VM)
- **Margin Reporting**: Summary views by netting set, cleared/bilateral split, and total portfolio

## Module Structure

```
margin/
├── mod.rs              # Module root, re-exports public types
├── aggregator.rs       # PortfolioMarginAggregator - main calculation engine
├── netting_set.rs      # NettingSet and NettingSetManager types
└── results.rs          # NettingSetMargin and PortfolioMarginResult types
```

### Components

| File | Description |
|------|-------------|
| `aggregator.rs` | Main orchestrator that organizes positions into netting sets, aggregates sensitivities, and calculates margin requirements |
| `netting_set.rs` | Types for managing netting sets - collections of positions that can offset each other |
| `results.rs` | Result types for netting set and portfolio-level margin calculations |

## Methodology

### Netting Set Organization

Positions are organized into netting sets based on their margin specifications:

1. **Cleared Trades**: Grouped by clearing house (e.g., LCH, CME, ICE)
2. **Bilateral Trades**: Grouped by counterparty and CSA agreement

```
Portfolio
├── Cleared: LCH
│   ├── IRS Position 1
│   └── IRS Position 2
├── Cleared: CME
│   └── IRS Position 3
└── Bilateral: BANK_A / CSA_001
    ├── CDS Position 1
    └── TRS Position 1
```

### Sensitivity Aggregation

Within each netting set, SIMM sensitivities are aggregated (netted) before calculating IM:

1. Collect sensitivities from each position in the netting set
2. Net sensitivities by risk class, currency, and tenor (e.g., two 5Y USD IR deltas net)
3. Calculate SIMM from the netted sensitivities

This provides proper risk offset recognition - offsetting positions reduce overall IM.

### Margin Calculations

**Initial Margin (IM)**:

- Calculated using ISDA SIMM from aggregated sensitivities
- Includes breakdown by risk class (IR, Credit, Equity, etc.)

**Variation Margin (VM)**:

- Sum of mark-to-market values across positions in the netting set
- Positive = we owe collateral, Negative = we receive collateral

**Total Margin**:

- IM + max(0, VM) - only positive VM increases total requirement

## Usage Examples

### Basic Portfolio Margin Calculation

```rust
use finstack_portfolio::margin::PortfolioMarginAggregator;
use finstack_core::dates::Date;

// Create portfolio with positions (each having a margin_spec)
let portfolio = Portfolio::new("MY_PORTFOLIO", Currency::USD)
    .with_positions(vec![irs_position, cds_position, trs_position]);

// Create aggregator from portfolio (auto-organizes into netting sets)
let mut aggregator = PortfolioMarginAggregator::from_portfolio(&portfolio);

// Calculate margin requirements
let as_of = Date::from_calendar_date(2025, time::Month::January, 15)?;
let result = aggregator.calculate(&portfolio, &market, as_of)?;

// Access results
println!("Total IM: {}", result.total_initial_margin);
println!("Total VM: {}", result.total_variation_margin);
println!("Total Margin: {}", result.total_margin);
println!("Netting Sets: {}", result.netting_set_count());
```

### Iterating Over Netting Set Results

```rust
for (netting_set_id, ns_margin) in result.iter() {
    println!(
        "{}: IM={}, VM={}, Positions={}",
        netting_set_id,
        ns_margin.initial_margin,
        ns_margin.variation_margin,
        ns_margin.position_count,
    );

    // Access SIMM breakdown by risk class
    for (risk_class, amount) in &ns_margin.im_breakdown {
        println!("  {}: {}", risk_class, amount);
    }
}
```

### Cleared vs Bilateral Split

```rust
let (cleared_margin, bilateral_margin) = result.cleared_bilateral_split();

println!("Cleared margin: {}", cleared_margin);
println!("Bilateral margin: {}", bilateral_margin);
```

### Manual Aggregator Construction

```rust
use finstack_portfolio::margin::{
    PortfolioMarginAggregator, NettingSetManager, NettingSetId,
};

// Create aggregator with specific base currency
let mut aggregator = PortfolioMarginAggregator::new(Currency::EUR);

// Add positions individually
for position in portfolio.positions.iter() {
    aggregator.add_position(position);
}

// Calculate
let result = aggregator.calculate(&portfolio, &market, as_of)?;
```

### Working with Netting Sets Directly

```rust
use finstack_portfolio::margin::{NettingSet, NettingSetManager, NettingSetId};
use finstack_margin::SimmSensitivities;

// Create netting set manager
let mut manager = NettingSetManager::new();

// Create and configure netting sets
let bilateral_id = NettingSetId::bilateral("COUNTERPARTY_A", "CSA_001");
let cleared_id = NettingSetId::cleared("LCH");

manager.get_or_create(bilateral_id.clone());
manager.get_or_create(cleared_id.clone());

// Add positions
let ns = manager.get_mut(&bilateral_id).expect("exists");
ns.add_position("SWAP_001".into());
ns.add_position("CDS_001".into());

// Manually merge sensitivities
let mut sensitivities = SimmSensitivities::new(Currency::USD);
sensitivities.add_ir_delta(Currency::USD, "5Y", 100_000.0);
ns.merge_sensitivities(&sensitivities);

// Check properties
assert!(!ns.is_cleared());
assert_eq!(ns.position_count(), 2);
```

### Accessing Aggregated Sensitivities

```rust
// After calculation, access aggregated sensitivities per netting set
for (ns_id, ns_margin) in result.iter() {
    if let Some(ref sensitivities) = ns_margin.sensitivities {
        // Access IR delta sensitivities
        for ((currency, tenor), delta) in &sensitivities.ir_delta {
            println!("  IR Delta {}/{}: {}", currency, tenor, delta);
        }

        // Access credit sensitivities
        for ((name, tenor), delta) in &sensitivities.credit_delta {
            println!("  Credit Delta {}/{}: {}", name, tenor, delta);
        }
    }
}
```

## Supported Instruments

The following instruments support margin aggregation (via the `Marginable` trait):

| Instrument | Sensitivity Type | Notes |
|------------|------------------|-------|
| `InterestRateSwap` | IR Delta (DV01 by tenor) | Full tenor bucketing |
| `CreditDefaultSwap` | Credit Delta (CS01) | By reference entity |
| `CDSIndex` | Credit Delta | Index-level sensitivity |
| `EquityTotalReturnSwap` | Equity Delta | By underlier |
| `FIIndexTotalReturnSwap` | IR Delta | Duration-based |
| `Repo` | IR Delta | Short-tenor sensitivity |

## Result Types

### `NettingSetMargin`

Margin results for a single netting set:

| Field | Type | Description |
|-------|------|-------------|
| `netting_set_id` | `NettingSetId` | Identifier for the netting set |
| `as_of` | `Date` | Calculation date |
| `initial_margin` | `Money` | IM requirement |
| `variation_margin` | `Money` | VM requirement |
| `total_margin` | `Money` | IM + max(0, VM) |
| `position_count` | `usize` | Number of positions |
| `im_methodology` | `ImMethodology` | SIMM, ClearingHouse, etc. |
| `sensitivities` | `Option<SimmSensitivities>` | Aggregated sensitivities |
| `im_breakdown` | `HashMap<String, Money>` | IM by risk class |

### `PortfolioMarginResult`

Portfolio-wide margin summary:

| Field | Type | Description |
|-------|------|-------------|
| `as_of` | `Date` | Calculation date |
| `base_currency` | `Currency` | Reporting currency |
| `total_initial_margin` | `Money` | Sum of IM across netting sets |
| `total_variation_margin` | `Money` | Sum of VM across netting sets |
| `total_margin` | `Money` | Total margin requirement |
| `by_netting_set` | `HashMap<NettingSetId, NettingSetMargin>` | Results by netting set |
| `total_positions` | `usize` | Positions with margin specs |
| `positions_without_margin` | `usize` | Positions excluded |

## Limitations

### Current Limitations

1. **Single Currency**: Portfolio aggregation assumes all positions use the same base currency. Cross-currency FX conversion for margin aggregation is not yet implemented.

2. **Simplified SIMM**: Uses the SIMM calculator from valuations crate which has simplified correlation handling. Full ISDA SIMM cross-bucket correlations not implemented.

3. **Static Netting Sets**: Positions are assigned to netting sets at aggregation time. Dynamic re-assignment based on changing CSA terms is not supported.

4. **No Collateral Tracking**: This module calculates margin requirements but does not track actual collateral posted or received.

5. **Conservative CCP IM**: Clearing house IM uses conservative estimates rather than actual CCP methodologies (which require API integration).

6. **No Regulatory Reporting**: Does not generate EMIR/Dodd-Frank/ASIC margin reports.

### Future Enhancements

1. **Cross-Currency Support**: Explicit FX conversion for multi-currency portfolios with currency-aware margin aggregation.

2. **Collateral Management Integration**: Track posted/received collateral, calculate excess/deficit, and generate margin calls.

3. **Full SIMM Correlation Matrix**: Complete ISDA SIMM v2.6 implementation with all cross-bucket and cross-risk-class correlations.

4. **Incremental Updates**: Efficient margin recalculation when individual positions change without full portfolio recalculation.

5. **MVA (Margin Valuation Adjustment)**: Calculate funding cost of future margin requirements for trade pricing.

6. **CCP API Integration**: Interface with LCH SMART, CME CORE for actual cleared margin calculations.

7. **Regulatory Reporting**: Generate standard margin reports for EMIR, Dodd-Frank, ASIC compliance.

8. **Collateral Optimization**: Optimal allocation of available collateral across netting sets to minimize funding cost.

9. **What-If Analysis**: Calculate margin impact of adding/removing positions before execution.

10. **Real-Time Monitoring**: Margin utilization alerts, threshold breach detection, and intraday margin projections.

## Relationship to Valuations Margin Module

This portfolio margin module builds on top of `finstack_valuations::margin`:

```
┌─────────────────────────────────────────────────┐
│           finstack_portfolio::margin            │
│   (Netting sets, aggregation, reporting)        │
├─────────────────────────────────────────────────┤
│           finstack_valuations::margin           │
│   (Marginable trait, SIMM calculator,           │
│    CSA specs, collateral schedules)             │
└─────────────────────────────────────────────────┘
```

- **Valuations**: Defines the `Marginable` trait, SIMM sensitivities, CSA specifications, and single-instrument margin calculations
- **Portfolio**: Organizes positions into netting sets, aggregates sensitivities, and provides portfolio-level reporting

## Testing

Run portfolio margin tests:

```bash
# Unit tests
cargo test -p finstack-portfolio margin::

# All portfolio tests
cargo test -p finstack-portfolio
```

## References

- [ISDA SIMM Methodology](https://www.isda.org/) - Standard Initial Margin Model
- [BCBS-IOSCO Margin Requirements](https://www.bis.org/) - Non-centrally cleared derivatives
- [EMIR Margin Rules](https://www.esma.europa.eu/) - EU margin requirements
- Finstack valuations margin module: `/finstack/valuations/src/margin/README.md`

# Finstack Market Data and Calibration Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan to extract market data and calibration functionality from the core and valuations crates into a clean, layered architecture. The new architecture separates concerns across five distinct layers, with the legacy valuations crate to be eventually deprecated and replaced.

Note: Layer 5 (analytics) is deferred to v2. Layers 1–4 (instruments, market-data, pricing, calibration) remain in v1.

## Architecture Overview

### Layered Design
```
┌─────────────────────────────────────┐
│    finstack-analytics (Layer 5)     │  ← High-level portfolio/risk analytics
├─────────────────────────────────────┤
│   finstack-calibration (Layer 4)    │  ← Market calibration algorithms
├─────────────────────────────────────┤
│     finstack-pricing (Layer 3)      │  ← Pricing engines and metrics
├─────────────────────────────────────┤
│   finstack-market-data (Layer 2)    │  ← Market context and curves
├─────────────────────────────────────┤
│   finstack-instruments (Layer 1)    │  ← Pure instrument definitions
├─────────────────────────────────────┤
│       finstack-core (Layer 0)       │  ← Core types and math
└─────────────────────────────────────┘
```

Scope note: Layer 5 is planned for v2. v1 delivers Layers 1–4.

### Dependency Flow
- Each layer depends only on layers below it
- No circular dependencies
- Clean interfaces between layers

## Layer 1: finstack-instruments

### Purpose
Pure data structures for financial instruments and their market quotes. No pricing logic, just data. All fixed income instruments leverage a comprehensive shared cashflow infrastructure for maximum reusability and consistency.

### Structure
```
finstack-instruments/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs               # Core traits (Identifiable, Attributable)
│   ├── quotes/
│   │   ├── mod.rs
│   │   ├── quote_types.rs      # Generic quote types
│   │   └── conversion.rs       # Quote to instrument conversions
│   ├── fixed_income/
│   │   ├── mod.rs
│   │   ├── cashflow/           # Shared cashflow infrastructure
│   │   │   ├── mod.rs
│   │   │   ├── schedules.rs    # Coupon schedule generation
│   │   │   ├── amortization.rs # Amortization schedules
│   │   │   ├── optionality.rs  # Call/put schedules
│   │   │   ├── fees.rs         # Fee structures
│   │   │   └── builder.rs      # Cashflow builder pattern
│   │   ├── deposit.rs          # Deposit struct + DepositQuote
│   │   ├── fra.rs              # FRA struct + FRAQuote
│   │   ├── future.rs           # IRFuture struct + FutureQuote
│   │   ├── swap.rs             # IRS struct + SwapQuote
│   │   ├── bond.rs             # Bond struct + BondQuote
│   │   ├── cds.rs              # CDS struct + CDSQuote
│   │   ├── loan/
│   │   │   ├── mod.rs
│   │   │   ├── term_loan.rs    # Term loan structures
│   │   │   ├── revolver.rs     # Revolving credit structures
│   │   │   └── ddtl.rs         # Delayed draw term loan
│   │   └── inflation/
│   │       ├── mod.rs
│   │       ├── inflation_swap.rs  # Inflation Swap structure
│   │       └── inflation_bond.rs  # Inflation Bond (ILB) structure
│   ├── options/
│   │   ├── mod.rs
│   │   ├── equity_option.rs    # Plain vanilla equity European/American options
│   │   ├── credit_option.rs    # Credit default options
│   │   ├── swaption.rs         # Swaption structures
│   │   └── cap_floor.rs        # Interest rate caps/floors

│   ├── equity/
│   │   ├── mod.rs
│   │   ├── stock.rs            # Single stock
│   │   ├── index.rs            # Equity index
│   │   └── etf.rs              # ETF structures
│   ├── fx/
│   │   ├── mod.rs
│   │   ├── spot.rs             # FX spot
│   │   ├── forward.rs          # FX forward
│   │   └── swap.rs             # FX swap
│   ├── structured/
│   │   ├── mod.rs
│   │   ├── convertible.rs      # Convertible bonds
│   │   ├── tranche.rs          # CDO/CLO tranches
│   │   └── waterfall.rs        # Waterfall structures
│   └── covenants/              # NOTE: Data structures only - evaluation logic in analytics layer
│       ├── mod.rs
│       ├── covenant_spec.rs    # Covenant specification structures (pure data)
│       ├── breach.rs           # Covenant breach tracking structures
│       └── types.rs            # Covenant types and consequences definitions
```

### Key Design Principles
1. **Pure Data**: Structures contain only data fields, no methods beyond constructors and converters
2. **Serializable**: All structures implement Serialize/Deserialize
3. **Immutable by Default**: Use builder patterns for complex construction
4. **Quote Integration**: Each instrument type has a corresponding quote type
5. **Shared Cashflow Infrastructure**: Comprehensive reusable cashflow components for all fixed income instruments
   - Flexible coupon schedule generation (fixed, floating, step-up, PIK, range)
   - Full amortization support (linear, custom schedules, bullets, step-remaining)
   - Call/put optionality with multiple exercise dates and make-whole provisions
   - Complex fee structures (upfront, periodic, exit, commitment fees)
   - Consistent day count and business day convention handling

### Benefits of Shared Cashflow Infrastructure
- **Consistency**: All fixed income instruments handle cashflows the same way
- **Reusability**: Complex cashflow logic written once, used everywhere
- **Flexibility**: Easy to add new cashflow features that all instruments can use
- **Maintainability**: Bug fixes and enhancements benefit all instruments
- **Testing**: Comprehensive cashflow testing covers all instrument types

### Example Implementation
```rust
// src/fixed_income/cashflow/mod.rs
// Core cashflow components used by all fixed income instruments

use serde::{Deserialize, Serialize};
use finstack_core::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashflowSpec {
    pub coupon_spec: CouponSpec,
    pub amortization: AmortizationSpec,
    pub fees: Vec<FeeSpec>,
    pub day_count: DayCount,
    pub business_day_convention: BusinessDayConvention,
    pub calendar: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallPutSchedule {
    pub call_dates: Vec<CallOption>,
    pub put_dates: Vec<PutOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallOption {
    pub exercise_date: Date,
    pub strike_price: Money,
    pub make_whole_spread: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PutOption {
    pub exercise_date: Date,
    pub strike_price: Money,
}

// src/fixed_income/swap.rs

use super::cashflow::{CashflowSpec, CouponSpec};
use serde::{Deserialize, Serialize};
use finstack_core::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterestRateSwap {
    pub id: String,
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub pay_leg: CashflowSpec,    // Uses shared cashflow infrastructure
    pub receive_leg: CashflowSpec, // Uses shared cashflow infrastructure
    pub attributes: Attributes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapQuote {
    pub maturity: Date,
    pub rate: f64,
    pub fixed_freq: Frequency,
    pub float_freq: Frequency,
    pub fixed_dc: DayCount,
    pub float_dc: DayCount,
    pub index: String,
    pub quote_type: SwapQuoteType,
    pub bid_ask_spread: Option<f64>,
    pub source: String,
    pub timestamp: DateTime,
}

impl SwapQuote {
    pub fn to_instrument(&self, id: String, base_date: Date, notional: Money) -> InterestRateSwap {
        let pay_leg = CashflowSpec {
            coupon_spec: CouponSpec::Fixed { 
                rate: self.rate, 
                frequency: self.fixed_freq, 
                day_count: self.fixed_dc 
            },
            amortization: AmortizationSpec::Bullet,
            fees: vec![],
            day_count: self.fixed_dc,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            calendar: "TARGET".to_string(),
        };
        
        let receive_leg = CashflowSpec {
            coupon_spec: CouponSpec::Floating { 
                index: self.index.clone(), 
                spread: 0.0, 
                frequency: self.float_freq 
            },
            amortization: AmortizationSpec::Bullet,
            fees: vec![],
            day_count: self.float_dc,
            business_day_convention: BusinessDayConvention::ModifiedFollowing,
            calendar: "TARGET".to_string(),
        };
        
        InterestRateSwap {
            id,
            effective_date: base_date,
            maturity_date: self.maturity,
            notional,
            pay_leg,
            receive_leg,
            attributes: Attributes::default(),
        }
    }
}

// src/fixed_income/bond.rs
// Bond using comprehensive cashflow infrastructure

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bond {
    pub id: String,
    pub issue_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub cashflow_spec: CashflowSpec,      // Coupon, amortization, fees
    pub optionality: Option<CallPutSchedule>, // Call/put features
    pub issue_price: Money,
    pub attributes: Attributes,
}

// src/fixed_income/loan/term_loan.rs
// Term loan with full cashflow support

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermLoan {
    pub id: String,
    pub origination_date: Date,
    pub maturity_date: Date,
    pub principal: Money,
    pub cashflow_spec: CashflowSpec,      // Interest, amortization, fees
    pub prepayment_option: Option<PrepaymentSchedule>,
    pub covenants: Vec<CovenantSpec>,
    pub attributes: Attributes,
}

// src/fixed_income/cds.rs
// CDS with premium leg cashflows

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditDefaultSwap {
    pub id: String,
    pub reference_entity: String,
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub premium_leg: CashflowSpec,        // Premium payments
    pub protection_leg: ProtectionSpec,   // Default protection
    pub recovery_rate: f64,
    pub attributes: Attributes,
}

// src/fixed_income/cashflow/builder.rs
// Comprehensive cashflow builder shared across all fixed income instruments

use finstack_core::prelude::*;

pub struct CashflowScheduleBuilder {
    notional: Option<Money>,
    issue_date: Option<Date>,
    maturity_date: Option<Date>,
    coupon_spec: Option<CouponSpec>,
    amortization: AmortizationSpec,
    fees: Vec<FeeSpec>,
    call_schedule: Option<CallSchedule>,
    put_schedule: Option<PutSchedule>,
}

impl CashflowScheduleBuilder {
    pub fn new() -> Self { /* ... */ }
    
    pub fn notional(mut self, amount: Money) -> Self { /* ... */ }
    
    pub fn coupon(mut self, spec: CouponSpec) -> Self { /* ... */ }
    
    pub fn with_amortization(mut self, spec: AmortizationSpec) -> Self { /* ... */ }
    
    pub fn with_call_schedule(mut self, schedule: CallSchedule) -> Self { /* ... */ }
    
    pub fn with_fees(mut self, fees: Vec<FeeSpec>) -> Self { /* ... */ }
    
    pub fn build(self) -> Result<CashflowSchedule> { /* ... */ }
}

pub enum CouponSpec {
    Fixed { rate: f64, frequency: Frequency, day_count: DayCount },
    Floating { index: String, spread: f64, frequency: Frequency, day_count: DayCount },
    StepUp { schedule: Vec<(Date, f64)>, frequency: Frequency, day_count: DayCount },
    Range { floor: f64, cap: f64, reference: String, frequency: Frequency },
    PIK { rate: f64, frequency: Frequency },  // Payment-in-kind
}

pub enum AmortizationSpec {
    Bullet,
    Linear { target: Money },
    Custom { schedule: Vec<(Date, Money)> },
    PercentPerPeriod { percent: f64 },
    StepRemaining { schedule: Vec<(Date, Money)> },  // Remaining balance at dates
}

pub enum FeeSpec {
    Upfront { amount: Money },
    Periodic { bps: f64, frequency: Frequency, base: FeeBase },
    Exit { amount: Money },
    Commitment { bps: f64, on_undrawn: bool },
}

pub enum FeeBase {
    Outstanding,  // Based on outstanding principal
    Original,     // Based on original principal
    Drawn,        // Based on drawn amount
    Undrawn,      // Based on undrawn commitment
}

// src/covenants/types.rs
// Covenant data structures for loans and structured products

/// Covenant specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantSpec {
    /// The covenant definition
    pub covenant: Covenant,
    /// Type of covenant
    pub covenant_type: CovenantType,
    /// Testing frequency
    pub test_frequency: Frequency,
    /// Grace period after breach
    pub cure_period_days: Option<i32>,
    /// Consequences if breached
    pub consequences: Vec<CovenantConsequence>,
}

/// Covenant breach tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantBreach {
    /// Covenant that was breached
    pub covenant_type: String,
    /// Date of the breach
    pub breach_date: Date,
    /// Actual value that caused the breach
    pub actual_value: Option<F>,
    /// Required threshold
    pub threshold: Option<F>,
    /// Cure deadline
    pub cure_deadline: Option<Date>,
    /// Whether the breach has been cured
    pub is_cured: bool,
    /// Applied consequences
    pub applied_consequences: Vec<CovenantConsequence>,
}

pub enum CovenantType {
    MaxDebtToEBITDA { threshold: F },
    MinInterestCoverage { threshold: F },
    MaxTotalLeverage { threshold: F },
    MinAssetCoverage { threshold: F },
    // ... other covenant types
}

// Example: Creating a complex bond with step-up coupon and amortization
let bond_cashflows = CashflowScheduleBuilder::new()
    .notional(Money::new(100_000_000.0, Currency::USD))
    .issue_date(Date::from_ymd(2024, 1, 1))
    .maturity_date(Date::from_ymd(2034, 1, 1))
    .coupon(CouponSpec::StepUp {
        schedule: vec![
            (Date::from_ymd(2024, 1, 1), 0.03),
            (Date::from_ymd(2027, 1, 1), 0.035),
            (Date::from_ymd(2030, 1, 1), 0.04),
        ],
        frequency: Frequency::SemiAnnual,
        day_count: DayCount::Thirty360,
    })
    .with_amortization(AmortizationSpec::Linear {
        target: Money::new(10_000_000.0, Currency::USD),
    })
    .with_call_schedule(CallSchedule {
        dates: vec![
            CallOption {
                exercise_date: Date::from_ymd(2029, 1, 1),
                strike_price: Money::new(102_000_000.0, Currency::USD),
                make_whole_spread: Some(0.005),
            },
        ],
    })
    .with_fees(vec![
        FeeSpec::Upfront { amount: Money::new(500_000.0, Currency::USD) },
        FeeSpec::Periodic { bps: 25.0, frequency: Frequency::Annual, base: FeeBase::Outstanding },
    ])
    .build()?;
```

## Layer 2: finstack-market-data

### Purpose
Market data context, term structures, surfaces, and indices. Infrastructure for storing and retrieving market data.

### Structure
```
finstack-market-data/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── context.rs              # Unified MarketContext
│   ├── traits.rs                # TermStructure, Discount, Forward, etc.
│   ├── primitives.rs            # MarketScalar, ScalarTimeSeries
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── validation.rs       # Validation helper functions
│   │   └── forward.rs          # Forward curve extraction traits
│   ├── bumping/
│   │   ├── mod.rs
│   │   ├── bump_spec.rs        # BumpSpec enum with all shock types
│   │   ├── bumped_curves.rs    # Bumped curve wrappers
│   │   ├── shock_scenarios.rs  # Predefined market shock scenarios
│   │   └── bump_engine.rs      # Orchestration for complex bumping
│   ├── term_structures/
│   │   ├── mod.rs
│   │   ├── discount_curve.rs   # Discount interest rate curve
│   │   ├── forward_curve.rs    # Forward curve
│   │   ├── hazard_curve.rs     # Credit Hazard Rate Curve
│   │   ├── inflation_curve.rs  # Inflation Curve
│   │   └── base_correlation.rs # CDS Tranche Base Correlation Curve
│   ├── surfaces/
│   │   ├── mod.rs
│   │   ├── vol_surface.rs      # Volatility Surfaces
│   │   └── local_vol.rs
│   ├── indices/
│   │   ├── mod.rs
│   │   ├── inflation_index.rs
│   │   ├── credit_index.rs      # CreditIndexData for CDS tranche pricing
│   │   └── equity_index.rs
│   └── fx/
│       ├── mod.rs
│       ├── fx_matrix.rs        # FX rate matrix
│       └── fx_provider.rs      # FX provider trait
```

### MarketContext Design
```rust
pub struct MarketContext {
    // Core term structures
    disc: HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    fwd: HashMap<CurveId, Arc<dyn Forward + Send + Sync>>,
    hazard: HashMap<CurveId, Arc<HazardCurve>>,
    inflation: HashMap<CurveId, Arc<InflationCurve>>,
    base_correlation: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
    
    // Surfaces
    vol_surfaces: HashMap<CurveId, Arc<VolSurface>>,
    local_vol_surfaces: HashMap<CurveId, Arc<LocalVolSurface>>,
    
    // Indices
    inflation_indices: HashMap<String, Arc<InflationIndex>>,
    credit_indices: HashMap<String, Arc<CreditIndexData>>,  // For CDS tranche pricing
    equity_indices: HashMap<String, Arc<EquityIndex>>,
    
    // FX
    fx: Option<Arc<FxMatrix>>,
    
    // Scalars and series
    prices: HashMap<CurveId, MarketScalar>,
    series: HashMap<CurveId, ScalarTimeSeries>,
    
    // Metadata
    as_of_date: Date,
    market_close_time: Option<DateTime>,
}

/// Credit index data for standardized indices (CDX, iTraxx, etc.)
pub struct CreditIndexData {
    /// Number of constituents (e.g., 125 for CDX IG)
    pub num_constituents: u16,
    /// Default recovery rate for the index
    pub recovery_rate: F,
    /// Hazard curve for the index as a whole
    pub index_credit_curve: Arc<HazardCurve>,
    /// Base correlation curve for tranches
    pub base_correlation_curve: Arc<BaseCorrelationCurve>,
    /// Optional individual issuer curves for heterogeneous modeling
    pub issuer_credit_curves: Option<HashMap<String, Arc<HazardCurve>>>,
}
```

### Key Features
- Unified context merging core and valuations contexts
- Efficient Arc-based sharing
- Comprehensive bumping/shocking capabilities
- Thread-safe access
- Credit index data support for CDS tranche pricing
- Forward curve extraction traits for equity, FX, and rates (in utils/forward.rs)
  - Provides trait-based forward pricing abstractions from market data
  - Implementations can vary based on market conventions
  - Used by calibration, pricing, and analytics layers

### Bumping Infrastructure
```rust
/// Comprehensive bump specification for market shocks
pub enum BumpSpec {
    /// Parallel shift in basis points for curves
    ParallelShift(ParallelShift),
    /// Multiplicative shock factor for prices/volatilities
    MultiplierShock(MultiplierShock),
    /// Spread shift in basis points for credit curves
    SpreadShift(ParallelShift),
    /// Percentage shift for inflation curves
    InflationShift(ParallelShift),
    /// Percentage shift for correlation values
    CorrelationShift(ParallelShift),
}

impl MarketContext {
    /// Create a bumped copy of the market context
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> Result<Self> {
        // Apply bumps to create new context with shocked market data
    }
}
```

## Layer 3: finstack-pricing

### Purpose
All pricing engines, valuation logic, and metric calculations. Completely separated from instrument definitions.

### Structure
```
finstack-pricing/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs                # Priceable, MetricCalculator traits
│   ├── error.rs                 # Pricing-specific errors
│   ├── results.rs               # PricingResult and basic result types
│   ├── models/                  # Pure mathematical models
│   │   ├── mod.rs
│   │   ├── black_scholes.rs    # Black-Scholes model for equity options
│   │   ├── black.rs             # Black model for interest rate derivatives
│   │   ├── garman_kohlhagen.rs  # FX option model
│   │   ├── sabr.rs              # SABR volatility model
│   │   ├── trees/
│   │   │   ├── mod.rs
│   │   │   ├── binomial.rs      # Cox-Ross-Rubinstein, Leisen-Reimer, etc.
│   │   │   ├── trinomial.rs     # Trinomial tree models
│   │   │   └── short_rate.rs    # Ho-Lee, Black-Derman-Toy
│   │   └── monte_carlo/
│   │       ├── mod.rs
│   │       ├── path_generator.rs # Path generation for various processes
│   │       ├── processes.rs      # GBM, mean-reverting, jump processes
│   │       └── random.rs         # Random number generation
│   ├── cashflow/
│   │   ├── mod.rs
│   │   └── generator.rs         # Cashflow generation from specs
│   ├── fixed_income/
│   │   ├── mod.rs
│   │   ├── deposit.rs           # Deposit pricing using discount curves
│   │   ├── fra.rs               # FRA pricing
│   │   ├── swap.rs              # Swap pricing (vanilla IRS, basis, cross-currency)
│   │   ├── bond.rs              # Bond pricing (analytical and tree-based for callables)
│   │   ├── cds.rs               # CDS pricing using hazard curves
│   │   └── loan.rs              # Loan pricing with prepayment models
│   ├── options/
│   │   ├── mod.rs
│   │   ├── equity_option.rs    # Equity options (uses Black-Scholes or trees)
│   │   ├── fx_option.rs        # FX options (uses Garman-Kohlhagen)
│   │   ├── swaption.rs         # Swaptions (uses Black or SABR)
│   │   ├── cap_floor.rs        # Caps/floors (uses Black or SABR)
│   │   └── credit_option.rs    # Credit options
│   ├── structured/
│   │   ├── mod.rs
│   │   ├── convertible.rs      # Convertible bonds (tree or MC)
│   │   └── tranche.rs          # CDO/CLO tranches
│   └── metrics/
│       ├── mod.rs
│       ├── fixed_income/
│       │   ├── mod.rs
│       │   ├── yield.rs         # Yield metrics: YTM, YTC, YTW
│       │   ├── spread.rs        # Spread metrics: Z-Spread, OAS, ASW, G-Spread
│       │   ├── price.rs         # Accrued, Clean Price, Dirty Price
│       │   ├── duration.rs      # Bond duration/convexity
│       ├── options/
│       │   ├── mod.rs
│       │   ├── greeks.rs        # Option Greeks
│       ├── risk/
│       │   ├── mod.rs
│       │   ├── dv01.rs          # Bucket DV01, CS01
│       │   ├── var.rs           # Value at Risk
│       │   ├── cvar.rs          # Conditional VaR
│       │   └── stress.rs        # Stress testing
│       └── cashflow/
│           ├── mod.rs
│           └── analysis.rs      # Cashflow analysis
```

### Pricing Architecture

The pricing layer uses a simplified structure where:
- **Models** contain pure mathematical formulas (Black-Scholes, SABR, trees, etc.)
- **Pricers** are instrument-specific and support multiple pricing methods
- Users can specify which method to use, or let the pricer choose intelligently

This avoids the complexity of having both "engines" and "pricers" while maintaining flexibility.

```rust
// Pricing method selection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PricingMethod {
    Auto,                    // Let the pricer choose the best method
    Analytical,              // Use closed-form formulas
    MonteCarlo { paths: usize, seed: Option<u64> },
    BinomialTree { steps: usize },
    TrinomialTree { steps: usize },
    FiniteDifference { grid_points: usize },
}

// results.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingResult {
    pub value: Money,
    pub as_of: Date,
    pub method_used: PricingMethod,  // Records which method was actually used
    pub calculation_time: Duration,
    pub warnings: Vec<String>,
}

// traits.rs
pub trait Priceable {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<PricingResult> {
        self.value_with_method(context, as_of, PricingMethod::Auto)
    }
    
    fn value_with_method(
        &self, 
        context: &MarketContext, 
        as_of: Date,
        method: PricingMethod
    ) -> Result<PricingResult>;
    
    fn calculate_metrics(
        &self, 
        context: &MarketContext, 
        as_of: Date, 
        metrics: &[MetricId]
    ) -> Result<MetricResults>;
}

// Example: Equity option pricer supporting multiple methods
// options/equity_option.rs
pub struct EquityOptionPricer;

impl EquityOptionPricer {
    /// Determines the default pricing method based on option characteristics
    /// This is the single source of truth for default method selection
    fn select_default_method(&self, option: &EquityOption) -> PricingMethod {
        if option.has_barriers() {
            PricingMethod::MonteCarlo { paths: 10_000, seed: None }
        } else if option.is_asian() {
            PricingMethod::MonteCarlo { paths: 10_000, seed: None }
        } else if option.exercise_style == ExerciseStyle::American {
            PricingMethod::BinomialTree { steps: 100 }
        } else if option.exercise_style == ExerciseStyle::European {
            PricingMethod::Analytical
        } else {
            // Fallback for complex/unrecognized types
            PricingMethod::MonteCarlo { paths: 10_000, seed: None }
        }
    }
    
    pub fn price(
        &self, 
        option: &EquityOption, 
        context: &MarketContext, 
        as_of: Date,
        method: PricingMethod
    ) -> Result<PricingResult> {
        match method {
            PricingMethod::Auto => {
                // Use the configured default for this option type
                let selected_method = self.select_default_method(option);
                self.price(option, context, as_of, selected_method)
            },
            PricingMethod::Analytical => {
                if option.exercise_style != ExerciseStyle::European {
                    return Err(PricingError::MethodNotSupported(
                        "Analytical pricing only available for European options"
                    ));
                }
                self.price_black_scholes(option, context, as_of)
            },
            PricingMethod::MonteCarlo { paths, seed } => {
                self.price_monte_carlo(option, context, as_of, paths, seed)
            },
            PricingMethod::BinomialTree { steps } => {
                self.price_binomial(option, context, as_of, steps)
            },
            _ => Err(PricingError::MethodNotSupported("Method not implemented for equity options"))
        }
    }
    
    fn price_black_scholes(&self, option: &EquityOption, context: &MarketContext, as_of: Date) -> Result<PricingResult> {
        // Use Black-Scholes model from models/black_scholes.rs
        let bs_model = BlackScholes::new(/* params */);
        let value = bs_model.price(/* ... */)?;
        Ok(PricingResult {
            value,
            method_used: PricingMethod::Analytical,
            // ...
        })
    }
    
    fn price_monte_carlo(&self, option: &EquityOption, context: &MarketContext, as_of: Date, paths: usize, seed: Option<u64>) -> Result<PricingResult> {
        // Use Monte Carlo path generator from models/monte_carlo/
        let path_gen = PathGenerator::new(/* params */);
        let value = path_gen.simulate(/* ... */)?;
        Ok(PricingResult {
            value,
            method_used: PricingMethod::MonteCarlo { paths, seed },
            // ...
        })
    }
    
    fn price_binomial(&self, option: &EquityOption, context: &MarketContext, as_of: Date, steps: usize) -> Result<PricingResult> {
        // Use binomial tree from models/trees/
        let tree = BinomialTree::new(steps);
        let value = tree.price(/* ... */)?;
        Ok(PricingResult {
            value,
            method_used: PricingMethod::BinomialTree { steps },
            // ...
        })
    }
}

// Simple registry for instrument pricers
pub struct PricingRegistry {
    pricers: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl PricingRegistry {
    pub fn new() -> Self {
        Self {
            pricers: HashMap::new(),
        }
    }
    
    /// Register default pricers
    pub fn register_defaults(&mut self) {
        self.register::<EquityOption, _>(EquityOptionPricer);
        self.register::<Bond, _>(BondPricer);
        self.register::<Swaption, _>(SwaptionPricer);
        self.register::<InterestRateSwap, _>(SwapPricer);
        // ... register other pricers
    }
    
    /// Register a custom pricer
    pub fn register<T, P>(&mut self, pricer: P) 
    where 
        T: 'static,
        P: Pricer<T> + 'static
    {
        self.pricers.insert(TypeId::of::<T>(), Box::new(pricer));
    }
    
    /// Price with default method (Auto)
    pub fn price<T>(&self, instrument: &T, context: &MarketContext, as_of: Date) -> Result<PricingResult>
    where 
        T: 'static
    {
        self.price_with_method(instrument, context, as_of, PricingMethod::Auto)
    }
    
    /// Price with specific method
    pub fn price_with_method<T>(&self, instrument: &T, context: &MarketContext, as_of: Date, method: PricingMethod) -> Result<PricingResult>
    where 
        T: 'static
    {
        let pricer = self.pricers.get(&TypeId::of::<T>())
            .ok_or(PricingError::NoPricerRegistered)?;
        // ... downcast and price with method
    }
}
```

### Default Pricing Methods

Pricing methods work at two simple levels:

1. **Built-in Defaults**: Each pricer has a `select_default_method()` function that chooses the appropriate method based on instrument characteristics
2. **Runtime Override**: Users can explicitly specify any pricing method when calling `price_with_method()`

```rust
// Example usage
use finstack_pricing::{PricingRegistry, PricingMethod};

// Create registry with built-in defaults
let mut registry = PricingRegistry::new();
registry.register_defaults();

// Price with built-in default (Auto selects based on instrument type)
let result = registry.price(&option, &market_context, as_of)?;
println!("Used method: {:?}", result.method_used);  // e.g., "BinomialTree { steps: 100 }"

// Override with specific method at runtime
let result_mc = registry.price_with_method(
    &option,
    &market_context, 
    as_of,
    PricingMethod::MonteCarlo { paths: 100_000, seed: Some(42) }
)?;

// The defaults are simple and predictable:
// - European vanilla options → Analytical (Black-Scholes)
// - American options → Binomial Tree
// - Barrier options → Monte Carlo
// - Asian options → Monte Carlo
// - Bonds without options → Analytical discounting
// - Callable/Putable bonds → Short-rate tree
```

### Implementation Strategy
1. Extract all pricing logic from current valuations crate
2. Create pure mathematical models in `/models` directory
3. Implement instrument-specific pricers that support multiple methods
4. Add `PricingMethod` enum for method selection (Auto, Analytical, MonteCarlo, Tree, etc.)
5. Each pricer implements a simple `select_default_method()` function
6. Users can override with any method at runtime via `price_with_method()`

## Layer 4: finstack-calibration

### Purpose
Market calibration algorithms that use the pricing layer to bootstrap curves and surfaces from market quotes.

### Structure
```
finstack-calibration/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── traits.rs                # Calibrator trait
│   ├── config.rs                # CalibrationConfig
│   ├── report.rs                # CalibrationReport
│   ├── error.rs                 # Calibration errors
│   ├── primitives.rs            # HashableFloat, constraints
│   ├── bootstrap/
│   │   ├── mod.rs
│   │   ├── yield_curve.rs       # Single-curve bootstrap
│   │   ├── multi_curve.rs       # Multi-curve bootstrap
│   │   ├── hazard_curve.rs      # Credit curve bootstrap
│   │   ├── inflation_curve.rs   # Inflation curve bootstrap
│   │   └── fx_curve.rs          # FX forward curve
│   ├── surface/
│   │   ├── mod.rs
│   │   ├── vol_surface.rs       # Volatility surface fitting
│   │   ├── sabr.rs              # SABR calibration
│   │   └── local_vol.rs         # Local volatility
│   ├── correlation/
│   │   ├── mod.rs
│   │   └── base_correlation.rs  # Base correlation fitting
│   ├── optimization/
│   │   ├── mod.rs
│   │   ├── global.rs             # Global optimization
│   │   ├── least_squares.rs      # Least squares fitting
│   │   └── maximum_likelihood.rs # MLE calibration
│   ├── common/
│   │   ├── mod.rs
│   │   ├── grouping.rs          # Quote grouping utilities
│   │   ├── identifiers.rs       # Curve ID generation
│   │   └── time.rs              # Time utilities
│   ├── dag/
│   │   ├── mod.rs
│   │   ├── dependency.rs        # Dependency analysis
│   │   └── scheduler.rs         # Calibration scheduling
│   └── orchestrator.rs          # End-to-end orchestration
```

### Calibration Architecture
```rust
// Import forward curve traits from market-data layer
use finstack_market_data::utils::forward::{ForwardPricer, EquityForward, FxForward, RatesForward};

// Using pricing layer for calibration
impl YieldCurveBootstrapper {
    pub fn calibrate(
        &self,
        quotes: &[SwapQuote],
        pricer: &PricingRegistry,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        let mut curve_builder = DiscountCurveBuilder::new();
        let mut residuals = HashMap::new();
        
        for quote in quotes {
            // Convert quote to instrument
            let swap = quote.to_instrument(
                format!("CALIB_{}", quote.maturity),
                self.base_date, 
                Money::new(1_000_000.0, self.currency)
            );
            
            // Solve for discount factor
            let df = self.solver.solve(|df| {
                let mut temp_context = base_context.clone();
                temp_context.update_discount_point(quote.maturity, df);
                
                // Use pricing layer
                pricer.price(&swap, &temp_context, self.base_date)
                    .map(|pv| pv.amount())
                    .unwrap_or(f64::MAX)
            }, initial_guess)?;
            
            curve_builder.add_point(quote.maturity, df);
            residuals.insert(format!("{}", quote.maturity), residual);
        }
        
        let curve = curve_builder.build()?;
        let report = CalibrationReport::success_with(residuals, iterations, "Bootstrap complete");
        
        Ok((curve, report))
    }
}
```

### Key Features
- Uses actual instrument pricing from pricing layer
- No duplication of pricing logic
- Supports sequential and global calibration
- Dependency-aware orchestration

## Layer 5: finstack-analytics (New High-Level Layer)

Status: Deferred to v2 (not part of the v1 cutover).

### Purpose
High-level analytics, portfolio management, risk analytics, and reporting. Replaces the legacy valuations crate.

### Structure
```
finstack-analytics/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── portfolio/
│   │   ├── mod.rs
│   │   ├── portfolio.rs         # Portfolio container
│   │   ├── position.rs          # Position management (quantity, cost basis, P&L)
│   │   ├── book.rs              # Trading book hierarchy
│   │   ├── aggregation.rs       # Risk aggregation
│   │   └── optimization.rs      # Portfolio optimization (FUTURE FEATURES)
│   ├── risk/
│   │   ├── mod.rs
│   │   ├── market_risk/
│   │   │   ├── mod.rs
│   │   │   ├── var.rs           # Value at Risk
│   │   │   ├── scenarios.rs     # Scenario analysis
│   │   │   └── stress.rs        # Stress testing
│   │   ├── credit_risk/
│   │   │   ├── mod.rs
│   │   │   ├── cva.rs           # CVA/DVA
│   │   │   └── exposure.rs      # Exposure profiles
│   ├── analytics/
│   │   ├── mod.rs
│   │   ├── cashflow/
│   │   │   ├── mod.rs
│   │   │   ├── projection.rs    # Cashflow projection
│   │   │   ├── analysis.rs      # Cashflow analysis
│   │   │   └── aggregation.rs   # Currency-preserving period aggregation
│   │   ├── scenario/
│   │   │   ├── mod.rs
│   │   │   ├── definition.rs    # Scenario definitions (combines market bumps + other params)
│   │   │   ├── engine.rs        # Scenario execution engine
│   │   │   ├── builder.rs       # Scenario builder
│   │   │   └── library.rs       # Pre-defined scenarios (Fed hike, crisis, etc.)
│   │   └── sensitivity/
│   │       ├── mod.rs
│   │       └── ladder.rs        # Risk ladder
│   ├── results/
│   │   ├── mod.rs
│   │   ├── valuation_result.rs  # ValuationResult with metrics
│   │   ├── covenant_report.rs   # Covenant check results
│   │   └── metadata.rs          # ExtendedResultsMeta
│   ├── reporting/
│   │   ├── mod.rs
│   │   ├── formats/
│   │   │   ├── mod.rs
│   │   │   ├── json.rs
│   │   │   ├── csv.rs
│   │   │   └── excel.rs
│   │   ├── templates/
│   │   │   ├── mod.rs
│   │   │   ├── risk_report.rs
│   │   │   └── performance_report.rs
│   │   └── visualization/
│   │       ├── mod.rs
│   │       └── charts.rs
│   └── covenants/
│       ├── mod.rs
│       ├── engine.rs            # Covenant evaluation engine
│       ├── evaluator.rs         # Custom evaluator functions
│       └── application.rs       # Consequence application logic
```

### Example High-Level API
```rust
use finstack_analytics::portfolio::Portfolio;
use finstack_analytics::risk::market_risk::VaR;
use finstack_analytics::performance::Returns;

// Create portfolio
let mut portfolio = Portfolio::new("Main Portfolio");
portfolio.add_position(position1);
portfolio.add_position(position2);

// Calculate metrics
let market_value = portfolio.market_value(&market_context, as_of)?;
let var_95 = VaR::calculate(&portfolio, &market_context, 0.95, 10)?;
let returns = Returns::calculate(&portfolio, start_date, end_date)?;

// Generate report
let report = RiskReport::new(&portfolio)
    .with_var(var_95)
    .with_stress_scenarios(scenarios)
    .generate(&market_context)?;
```

## Key Architectural Decisions

### 1. Credit Index Data Integration
Credit index data (`CreditIndexData`) is integrated directly into the unified `MarketContext` in `finstack-market-data`, providing:
- Support for standardized credit indices (CDX, iTraxx, etc.)
- Individual issuer curves for heterogeneous portfolio modeling
- Base correlation curves for tranche pricing
- Seamless access alongside other market data

### 2. Comprehensive Bumping Infrastructure
The bumping/shocking infrastructure lives in `finstack-market-data/src/bumping/` with:
- `BumpSpec` enum supporting all shock types (parallel, multiplier, spread, inflation, correlation)
- Bumped curve wrappers for efficient shocked market data
- Predefined shock scenarios for regulatory and stress testing
- The `bump()` method on `MarketContext` for creating shocked scenarios

### 3. Covenant Data Structures
Covenant specifications (`CovenantSpec`, `CovenantBreach`) are placed in `finstack-instruments/src/covenants/` as they are:
- Pure data structures defining covenant terms
- Used by multiple instrument types (loans, bonds, structured products)
- Serializable for storage and transmission
- **NOTE**: Contains only data definitions, no evaluation logic

The covenant *engine* for evaluation lives in `finstack-analytics/src/covenants/` as it requires:
- Metric calculation capabilities
- Complex evaluation logic
- Consequence application
- **NOTE**: All covenant evaluation and testing logic resides here

### 4. Results and Aggregation in Analytics
Results envelopes (`ValuationResult`, `CovenantReport`) and cashflow aggregation are placed in `finstack-analytics` because they:
- Represent high-level analysis outputs
- Require cross-instrument aggregation
- Support portfolio-level reporting
- Include metadata about calculation context

### 5. Forward Curve Traits in Market Data
Forward curve extraction traits (`ForwardPricer`, `EquityForward`, `FxForward`, `RatesForward`) are placed in `finstack-market-data/src/utils/forward.rs` rather than in calibration because:
- They are general market data utilities, not calibration-specific
- Multiple layers (pricing, analytics, calibration) benefit from these abstractions
- Trait-based design allows for different implementations based on market conventions
- Reduces coupling - other layers don't need to depend on calibration for basic utilities
- Maintains better cohesion with other market data extraction functions

### 6. Simple and Flexible Pricing Architecture
The pricing layer uses a streamlined structure without redundant "engines":
- Pure mathematical models in `finstack-pricing/src/models/` contain only formulas and algorithms
- Instrument-specific pricers support multiple pricing methods via a `PricingMethod` enum
- **Two-level simplicity**:
  - Built-in defaults: Each pricer's `select_default_method()` chooses based on instrument characteristics
  - Runtime override: Users can specify any method via `price_with_method()`
- No complex configuration layers or files - just sensible defaults and runtime flexibility
- The `PricingResult` includes which method was actually used for transparency
- Eliminates confusion between "engines" and "pricers" for the same instruments
- Cleaner testing of mathematical correctness vs. integration testing
- Aligns with preference for dedicated model code location while maintaining flexibility

### 7. Position Management in Analytics
Position management is placed in `finstack-analytics/src/portfolio/` including:
- Position tracking with quantity, cost basis, and P&L calculation
- Book hierarchy for organizing positions
- Aggregation logic for portfolio-level metrics
- This keeps all portfolio management concerns together at the highest layer

### 8. Scenario Definitions in Analytics
Scenario definitions live in `finstack-analytics/src/analytics/scenario/` because:
- Scenarios combine multiple market bumps with other parameters
- They require orchestration across different market data types
- Pre-defined scenario libraries (Fed scenarios, crisis scenarios) are application-level concerns
- Scenarios may include non-market parameters (operational assumptions, etc.)

### 9. Monte Carlo Path Generation in Pricing
Monte Carlo path generators are in `finstack-pricing/src/models/monte_carlo/` because:
- Path generation is a core pricing capability
- Used by multiple pricing engines (options, structured products)
- Tightly coupled with stochastic process definitions
- May be needed by calibration layer for certain techniques

### 10. Structured Products Remain in Instruments
The empty `structured-credit` crate will be removed, with all structured product definitions remaining in `finstack-instruments/src/structured/` for:
- Consistency with other instrument types
- Simpler dependency graph
- All instrument definitions in one place

## Migration Plan

### Phase 1: Foundation (Weeks 1-2)
1. Create `finstack-instruments` crate
   - Define all instrument structures (including structured products)
   - Define all quote structures
   - Implement builders and converters
   - Add comprehensive tests
   - Remove empty `structured-credit` crate

2. Create `finstack-market-data` crate
   - Move interpolation to `finstack-core::math::interp`
   - Extract market data types from core
   - Merge MarketContext implementations
   - Move credit index from valuations
   - Implement forward curve traits

### Phase 2: Pricing Layer (Weeks 3-4)
3. Create `finstack-pricing` crate
   - Define pricing traits and PricingResult type
   - Create models directory with pure mathematical models
   - Extract pricing logic from valuations into instrument-specific pricers
   - Implement pricing registry
   - Create pricers for all instruments (directly using models)
   - Implement cashflow generator
   - Add Monte Carlo path generators in models
   - Add comprehensive pricing tests

### Phase 3: Calibration (Weeks 5-6)
4. Create `finstack-calibration` crate
   - Extract calibration from valuations
   - Update to use pricing layer
   - Implement orchestrator
   - Add calibration tests

### Phase 4: Analytics Layer — Deferred to v2
This phase is moved out of v1. The following items are retained here as the v2 roadmap.
5. Create `finstack-analytics` crate
   - Implement portfolio management with position tracking
   - Add book hierarchy and aggregation
   - Create scenario definitions and engine
   - Add risk analytics (VaR, stress testing)
   - Implement covenant evaluation engine
   - Build reporting framework
   - Create ValuationResult and extended metadata

### Phase 5: Bindings Rebuild (Weeks 9-10)
6. Rebuild Python bindings from scratch
   - Complete redesign using new crate structure
   - No backward compatibility constraints
   - Modern PyO3 patterns and Pydantic v2 models
   - Focus on ergonomic Python API

7. Rebuild WASM bindings from scratch
   - Complete redesign for new architecture
   - Optimize for browser performance
   - TypeScript definitions from the start
   - Tree-shakeable modules

### Phase 6: Migration and Testing (Weeks 11-12)
8. Migrate existing code
   - Update all examples
   - Update all tests
   - Update documentation
   - Performance testing

9. Deprecation plan
   - Mark legacy valuations as deprecated
   - Provide migration guide
   - Plan removal timeline

## Benefits

### 1. Clean Architecture
- Clear separation of concerns
- Well-defined layer boundaries
- No circular dependencies

### 2. Maintainability
- Each layer can be developed independently
- Changes don't cascade across layers
- Easy to understand and modify

### 3. Performance
- Instrument structures are lightweight PODs
- Pricing can be optimized independently
- Parallel calibration possible

### 4. Flexibility
- Multiple pricing engines per instrument
- Pluggable calibration strategies
- Extensible analytics framework

### 5. Testing
- Each layer testable in isolation
- Mock implementations easy to create
- Comprehensive test coverage possible

## Risks and Mitigations

### Risk 1: Breaking Changes
**Mitigation**: 
- Provide compatibility layer initially
- Gradual migration with deprecation warnings
- Comprehensive migration guide

### Risk 2: Performance Regression
**Mitigation**:
- Benchmark before and after
- Profile critical paths
- Optimize hot spots

### Risk 3: Complexity
**Mitigation**:
- Clear documentation
- Examples for common use cases
- Training materials for team

## Success Criteria

1. **Architecture Goals**
   - Clean layer separation achieved
   - No pricing logic duplication
   - All tests passing

2. **Performance Goals**
   - No regression in pricing speed
   - Calibration at least as fast
   - Memory usage reasonable

3. **Usability Goals**
   - Clear, intuitive APIs
   - Comprehensive documentation
   - Migration path documented

## Example Code After Refactoring

```rust
use finstack_instruments::fixed_income::{InterestRateSwap, SwapQuote};
use finstack_market_data::MarketContext;
use finstack_pricing::{PricingRegistry, Priceable};
use finstack_calibration::{CalibrationOrchestrator, CalibrationConfig};
use finstack_analytics::portfolio::Portfolio;
use finstack_analytics::risk::VaR;

// Setup pricing
let mut pricing = PricingRegistry::new();
pricing.register_defaults(); // Register all default pricers

// Calibrate market from quotes
let quotes = vec![
    SwapQuote { maturity: date1, rate: 0.02, ... },
    SwapQuote { maturity: date2, rate: 0.025, ... },
];

let orchestrator = CalibrationOrchestrator::new(base_date, Currency::USD)
    .with_pricing(pricing.clone());
let (market_context, report) = orchestrator.calibrate_market(&quotes)?;

// Create and price instruments
let swap = InterestRateSwap::builder()
    .id("SWAP_001")
    .maturity_date(Date::from_ymd(2026, 12, 31))
    .fixed_rate(0.03)
    .build()?;

// Price with default method (Auto)
let pv = pricing.price(&swap, &market_context, base_date)?;

// Create an equity option
let option = EquityOption::builder()
    .underlying("AAPL")
    .strike(Money::new(150.0, Currency::USD))
    .maturity(Date::from_ymd(2025, 6, 30))
    .option_type(OptionType::Call)
    .exercise_style(ExerciseStyle::American)
    .build()?;

// Price with different methods
let pv_auto = pricing.price(&option, &market_context, base_date)?;  // Auto-selects binomial
let pv_mc = pricing.price_with_method(
    &option, 
    &market_context, 
    base_date,
    PricingMethod::MonteCarlo { paths: 100_000, seed: Some(42) }
)?;
let pv_tree = pricing.price_with_method(
    &option,
    &market_context,
    base_date, 
    PricingMethod::BinomialTree { steps: 200 }
)?;

println!("Auto pricing ({}): {}", pv_auto.method_used, pv_auto.value);
println!("Monte Carlo pricing: {}", pv_mc.value);
println!("Binomial tree pricing: {}", pv_tree.value);

// Portfolio analytics
let mut portfolio = Portfolio::new("Trading Book");
portfolio.add_instrument(Box::new(swap), 10_000_000.0); // $10M notional

let market_value = portfolio.market_value(&market_context, &pricing, base_date)?;

println!("Portfolio Market Value: {}", market_value);
```

## Conclusion

This refactoring plan provides a clean, maintainable architecture that:
- Separates concerns across well-defined layers
- Eliminates pricing logic duplication
- Provides flexibility for future enhancements
- Maintains high performance
- Enables comprehensive testing
- Preserves all critical functionality from existing codebase

Versioning note: v1 delivers Layers 1–4 (instruments, market-data, pricing, calibration). Layer 5 (analytics) ships in v2.

The migration can be done incrementally with minimal disruption to existing users while providing significant long-term benefits for maintainability and extensibility.

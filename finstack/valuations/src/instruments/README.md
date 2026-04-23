# Instruments Module

Comprehensive instrument library providing 40+ financial instrument types with deterministic pricing, risk metrics, and JSON interoperability.

## Overview

The instruments module is the core of Finstack's valuation capabilities, providing:

- **40+ Instrument Types**: Fixed income, derivatives (equity/FX/rates/credit), structured products, and private markets instruments
- **Unified Trait Interface**: All instruments implement the `Instrument` trait for consistent pricing and risk calculation
- **JSON Import/Export**: Stable, schema-versioned serialization for all instruments
- **Rich Metrics**: Instrument-specific risk metrics (Greeks, duration, spreads, yields)
- **Currency Safety**: Explicit currency handling with no implicit conversions
- **Determinism**: Decimal-based numerics ensure reproducible results

## Curve Trait Coverage

Curve-driven instruments expose their market data requirements through common traits:

- **CurveDependencies**: Aggregated discount/forward/credit curves for risk calculators.

Instruments with optional discounting (e.g., private markets funds) still price without curves but won’t participate in curve-based sensitivities until a curve is supplied.

## Directory Structure

```
instruments/
├── mod.rs                    # Module exports and re-exports
├── json_loader.rs           # JSON serialization/deserialization infrastructure
├── pricing_overrides.rs     # Market quote overrides for pricing
├── README.md                # This file
│
├── common/                  # Shared functionality across instruments
│   ├── traits.rs           # Core Instrument trait and Attributes
│   ├── discountable.rs     # NPV calculation interface
│   ├── parameters/         # Shared parameter types (legs, schedules, options)
│   ├── models/             # Pricing models (Black-Scholes, trees, MC, volatility)
│   ├── mc/                 # Monte Carlo engine (processes, discretization, payoffs)
│   └── pricing.rs          # Generic pricing patterns
│
├── bond/                    # Fixed-rate, floating-rate, callable/putable bonds
├── convertible/             # Convertible bonds with equity conversion
├── inflation_linked_bond/   # Inflation-indexed bonds (TIPS, linkers)
├── term_loan/               # Term loans with amortization
├── revolving_credit/        # Revolving credit facilities
│
├── irs/                     # Interest rate swaps (fixed vs floating)
├── basis_swap/              # Basis swaps (floating vs floating)
├── inflation_swap/          # Inflation swaps (fixed vs inflation index)
├── fra/                     # Forward rate agreements
├── swaption/                # Options on interest rate swaps
├── ir_future/               # Interest rate futures
├── cms_option/              # Constant maturity swap options
├── deposit/                 # Cash deposits
├── repo/                    # Repurchase agreements
│
├── cds/                     # Credit default swaps (single-name)
├── cds_index/               # CDS indices (CDX, iTraxx)
├── cds_tranche/             # Tranched CDS index positions
├── cds_option/              # Options on CDS spreads
├── structured_credit/       # ABS, RMBS, CMBS, CLO with prepayment/default models
│
├── equity/                  # Equity spot positions
├── equity_option/           # Vanilla equity options (European/American)
├── asian_option/            # Asian options (averaging)
├── barrier_option/          # Barrier options (knock-in/knock-out)
├── lookback_option/         # Lookback options (floating/fixed strike)
├── variance_swap/           # Variance and volatility swaps
│
├── fx_spot/                 # FX spot trades
├── fx_swap/                 # FX swaps (near/far legs)
├── fx_option/               # Vanilla FX options
├── fx_barrier_option/       # FX barrier options
├── quanto_option/           # Quanto options (cross-currency payoff)
│
├── autocallable/            # Autocallable notes
├── cliquet_option/          # Cliquet/ratchet options
├── range_accrual/           # Range accrual notes
│
├── trs/                     # Total return swaps (equity and fixed income index)
├── basket/                  # Basket instruments (multi-underlying)
└── private_markets_fund/    # Private markets fund vehicles
```

## Instrument Categories

### Fixed Income

- **Bond**: Fixed/floating rate bonds, callable/putable, amortizing
- **ConvertibleBond**: Bonds with equity conversion features
- **InflationLinkedBond**: Inflation-indexed bonds (TIPS-style)
- **TermLoan**: Leveraged loans with amortization schedules
- **RevolvingCredit**: Revolving credit facilities with draw/repay dynamics

### Interest Rate Derivatives

- **InterestRateSwap**: Fixed vs floating rate swaps
- **BasisSwap**: Floating vs floating (different tenors/indices)
- **InflationSwap**: Fixed vs inflation index returns
- **ForwardRateAgreement**: Single-period forward rate locks
- **Swaption**: Options on interest rate swaps (European/Bermudan)
- **InterestRateFuture**: Exchange-traded rate futures
- **CmsOption**: Options on CMS rates
- **Deposit**: Cash deposits with simple interest
- **Repo**: Repurchase agreements with collateral

### Credit Derivatives

- **CreditDefaultSwap**: Single-name CDS (protection buyer/seller)
- **CDSIndex**: CDS on credit indices (CDX, iTraxx)
- **CDSTranche**: Tranched index positions
- **CDSOption**: Options on CDS spreads
- **StructuredCredit**: ABS/RMBS/CMBS/CLO with behavioral models

### Equity Derivatives

- **Equity**: Spot equity positions
- **EquityOption**: Vanilla calls/puts (European/American)
- **AsianOption**: Path-dependent averaging options
- **BarrierOption**: Knock-in/knock-out barriers
- **LookbackOption**: Path-dependent lookback options
- **VarianceSwap**: Variance/volatility swaps

### FX Derivatives

- **FxSpot**: FX spot trades
- **FxSwap**: FX swaps with near/far legs
- **FxOption**: Vanilla FX options
- **FxBarrierOption**: FX barrier options
- **QuantoOption**: Cross-currency quanto options

### Exotic Options

- **Autocallable**: Autocallable structured notes
- **CliquetOption**: Cliquet/ratchet options
- **RangeAccrual**: Range accrual notes

### Other

- **EquityTotalReturnSwap**: Total return swaps on equities
- **FIIndexTotalReturnSwap**: Total return swaps on fixed income indices
- **Basket**: Multi-underlying basket instruments
- **PrivateMarketsFund**: Private markets fund vehicles

## Core Architecture

### The Instrument Trait

All instruments implement the `Instrument` trait defined in `common/traits.rs`:

```rust
pub trait Instrument: Send + Sync {
    // Identity
    fn id(&self) -> &str;
    fn key(&self) -> InstrumentType;

    // Metadata
    fn attributes(&self) -> &Attributes;
    fn attributes_mut(&mut self) -> &mut Attributes;

    // Pricing
    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money>;
    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult>;

    // Instrument lifecycle
    fn expiry(&self) -> Option<Date>;  // Maturity/expiry date for theta, roll calculations

    // Market data introspection (for attribution)
    fn market_dependencies(&self) -> MarketDependencies;
    fn fx_exposure(&self) -> Option<(Currency, Currency)>;
    fn dividend_schedule_id(&self) -> Option<CurveId>;

    // Trait object support
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn Instrument>;
}
```

**Note**: The `expiry()` method returns `Some(date)` for instruments with a defined expiry/maturity (bonds, swaps, options) or `None` for instruments without a clear expiry (e.g., equity spot positions). This is used by theta calculations to cap the roll date.

### Attributes and Selection

Instruments carry metadata via the `Attributes` type for categorization and scenario selection:

```rust
let attrs = Attributes::new()
    .with_tag("high-yield")
    .with_tag("energy")
    .with_meta("sector", "oil-gas")
    .with_meta("rating", "BB+");

// Selector matching for scenario application
assert!(attrs.matches_selector("tag:energy"));
assert!(attrs.matches_selector("meta:rating=BB+"));
assert!(attrs.matches_selector("*"));  // wildcard
```

### Typical Instrument Structure

Most instruments follow this pattern:

```
instrument_name/
├── mod.rs          # Module documentation, exports, basic tests
├── types.rs        # Core struct definition with builder pattern
├── pricer.rs       # Pricer implementation (Pricer<T> trait)
├── cashflows.rs    # Cashflow generation (if applicable)
└── metrics/        # Instrument-specific risk metrics
    ├── mod.rs
    ├── delta.rs
    ├── gamma.rs
    └── ...
```

## Key Features

### 1. JSON Import/Export

All instruments support JSON serialization via `InstrumentEnvelope`:

```rust
// Load from JSON
let instrument = InstrumentEnvelope::from_path("bond.json")?;

// Load from string
let json = r#"{
    "schema": "finstack.instrument/1",
    "instrument": {
        "type": "bond",
        "spec": {
            "id": "BOND-001",
            "notional": { "amount": "1000000", "currency": "USD" },
            "issue": "2024-01-01",
            "maturity": "2034-01-01",
            "cashflow_spec": {
                "Fixed": {
                    "coupon_type": "Cash",
                    "rate": 0.05,
                    "freq": { "Months": 6 },
                    "dc": "Thirty360",
                    "bdc": "following",
                    "calendar_id": null,
                    "stub": "None"
                }
            },
            "discount_curve_id": "USD-OIS"
        }
    }
}"#;
let instrument = InstrumentEnvelope::from_str(json)?;
```

**Schema Versioning**: All JSON uses versioned schemas (`finstack.instrument/1`) for forward-compatible evolution.

**Strict Validation**: `deny_unknown_fields` ensures typos and unknown fields are caught at deserialization.

### 2. Pricing

Fast NPV-only calculation:

```rust
let market = MarketContext::new();
let as_of = Date::from_ymd(2025, 1, 1)?;

// Fast path: NPV only (no metrics)
let pv = bond.value(&market, as_of)?;
assert_eq!(pv.currency(), Currency::USD);
```

NPV + risk metrics:

```rust
// Request specific metrics
let metrics = vec![MetricId::Ytm, MetricId::DurationMod, MetricId::Dv01];
let result = bond.price_with_metrics(&market, as_of, &metrics)?;

// Access results
println!("NPV: {}", result.value);
println!("YTM: {:.2}%", result.measures["ytm"] * 100.0);
println!("DV01: ${:.2}", result.measures["dv01"]);
```

### 3. Pricing Overrides

All instruments support market quote overrides via `PricingOverrides`:

```rust
let bond = Bond::fixed(/* ... */)
    .with_pricing_overrides(
        PricingOverrides::none()
            .with_quoted_clean_price(99.5)        // Override model price
            .with_ytm_bump_decimal(1e-4)   // 1bp bump for convexity
    );
```

Common overrides:

- `quoted_clean_price`: Bond prices
- `implied_volatility`: Option vols (overrides surface)
- `cds_quote_bp`: CDS spreads
- `upfront_payment`: Convertibles, CDS
- `adaptive_bumps`: Dynamic bump sizing for Greeks

### 4. Shared Parameter Types

Common leg and schedule types in `common/parameters`:

- **FixedLegSpec**: Fixed coupon leg parameters
- **FloatLegSpec**: Floating rate leg parameters
- **ProtectionLegSpec**: CDS protection leg
- **PremiumLegSpec**: CDS premium leg
- **TotalReturnLegSpec**: Total return leg for TRS
- **FinancingLegSpec**: Financing leg for TRS
- **ScheduleSpec**: Generic cashflow schedule generation
- **OptionMarketParams**: Spot, vol, dividend yield, rates
- **ExerciseStyle**: European, American, Bermudan

### 5. Pricing Models

Located in `common/models/`:

#### Closed-Form Models

- Black-Scholes for vanilla options
- Heston semi-analytic for stochastic vol
- Asian, Barrier, Lookback formulas
- Quanto adjustments

#### Volatility Models

- Black (log-normal)
- SABR (stochastic alpha-beta-rho) with calibration

#### Tree Models

- Binomial (CRR, JR, Tian, LR)
- Trinomial for short rates
- Multi-factor trees for rates + equity

#### Monte Carlo (`common/mc/`)

- **Processes**: GBM, Heston, CIR, OU, Jump Diffusion, Schwartz-Smith
- **Discretization**: Euler, Milstein, Exact (GBM/HW1F), QE (Heston/CIR)
- **Payoffs**: Vanilla, Asian, Barrier, Lookback, Autocallable, etc.
- **Variance Reduction**: Antithetic, control variates, moment matching
- **Greeks**: Pathwise, likelihood ratio, finite difference

### 6. Metrics

Instrument-specific metrics in each `instrument/metrics/` directory. Common metrics:

**Fixed Income**:

- YTM (yield to maturity)
- Duration (Macaulay, Modified)
- Convexity
- DV01
- CS01 (credit spread sensitivity)
- Z-spread, I-spread, OAS

**Options**:

- Delta, Gamma, Vega, Theta, Rho
- Volga, Vanna
- Charm, Vomma

**Swaps**:

- Par rate
- DV01 (parallel and bucketed)
- Annuity

## Usage Examples

### Creating a Fixed-Rate Bond

```rust
use finstack_valuations::instruments::{Bond, Instrument};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::Date;
use time::Month;

let bond = Bond::fixed(
    "BOND-001",
    Money::new(1_000_000.0, Currency::USD),
    0.05,  // 5% coupon
    Date::from_calendar_date(2025, Month::January, 1)?,
    Date::from_calendar_date(2035, Month::January, 1)?,
    "USD-OIS",  // discount curve
);

// Price with metrics
let market = MarketContext::new();
let as_of = Date::from_calendar_date(2025, Month::January, 1)?;
let metrics = vec![MetricId::Ytm, MetricId::DurationMod, MetricId::Dv01];
let result = bond.price_with_metrics(&market, as_of, &metrics)?;
```

### Creating an Interest Rate Swap

```rust
use finstack_valuations::instruments::{InterestRateSwap, PayReceive};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

let swap = InterestRateSwap::builder()
    .id("IRS-001")
    .notional(Money::new(10_000_000.0, Currency::USD))
    .start_date(Date::from_ymd(2025, 1, 1)?)
    .maturity_date(Date::from_ymd(2030, 1, 1)?)
    .fixed_rate(0.03)  // 3% fixed
    .direction(PayReceive::Pay)  // Pay fixed, receive floating
    .discount_curve_id("USD-OIS")
    .forward_curve_id("USD-SOFR")
    .build()?;
```

### Creating an Equity Option

```rust
use finstack_valuations::instruments::{EquityOption, OptionType};
use finstack_core::currency::Currency;

let option = EquityOption::builder()
    .id("AAPL-CALL-170")
    .underlying("AAPL")
    .option_type(OptionType::Call)
    .strike(170.0)
    .expiry(Date::from_ymd(2025, 6, 20)?)
    .quantity(100.0)  // 1 contract
    .settlement_currency(Currency::USD)
    .vol_surface_id("AAPL-VOL")
    .discount_curve_id("USD-OIS")
    .build()?;
```

### Creating a CDS

```rust
use finstack_valuations::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwapBuilder, PayReceive, PremiumLegSpec, ProtectionLegSpec,
    RECOVERY_SENIOR_UNSECURED,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use rust_decimal::Decimal;

let convention = CDSConvention::IsdaNa;
let cds = CreditDefaultSwap::builder()
    .id("CDS-CORP-001")
    .notional(Money::new(10_000_000.0, Currency::USD))
    .side(PayReceive::PayFixed)
    .convention(convention)
    .premium(PremiumLegSpec {
        start: Date::from_ymd(2025, 1, 1)?,
        end: Date::from_ymd(2030, 1, 1)?,
        freq: convention.frequency(),
        stub: convention.stub_convention(),
        bdc: convention.business_day_convention(),
        calendar_id: Some(convention.default_calendar().to_string()),
        dc: convention.day_count(),
        spread_bp: Decimal::try_from(100.0)?,
        discount_curve_id: CurveId::new("USD-OIS"),
    })
    .protection(ProtectionLegSpec {
        credit_curve_id: CurveId::new("CORP-HAZARD"),
        recovery_rate: RECOVERY_SENIOR_UNSECURED,
        settlement_delay: convention.settlement_delay(),
    })
    .pricing_overrides(PricingOverrides::default())
    .attributes(Attributes::new())
    .build()?;
```

### Loading from JSON

```rust
use finstack_valuations::instruments::json_loader::InstrumentEnvelope;

// From file
let instrument = InstrumentEnvelope::from_path("portfolio/bond_001.json")?;

// From string
let json = r#"{
    "schema": "finstack.instrument/1",
    "instrument": {
        "type": "equity_option",
        "spec": { /* ... */ }
    }
}"#;
let instrument = InstrumentEnvelope::from_str(json)?;

// Price it
let pv = instrument.value(&market, as_of)?;
```

## How to Add a New Instrument

Follow these steps to add a new instrument type to the library:

### 1. Create the Module Directory

```bash
mkdir finstack/valuations/src/instruments/my_instrument
```

### 2. Define the Instrument Type (`types.rs`)

```rust
//! My instrument type definition.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use serde::{Deserialize, Serialize};
use crate::instruments::{Attributes, Instrument};
use crate::instruments::PricingOverrides;

/// My custom instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MyInstrument {
    pub id: InstrumentId,
    pub notional: Money,
    pub maturity: Date,
    pub discount_curve_id: CurveId,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,

    // Instrument-specific fields
    pub some_parameter: f64,
}

impl MyInstrument {
    /// Create a new instance (provide a simple constructor).
    pub fn new(
        id: impl Into<InstrumentId>,
        notional: Money,
        maturity: Date,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            notional,
            maturity,
            discount_curve_id: discount_curve_id.into(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
            some_parameter: 0.0,
        }
    }

    /// Builder pattern (optional but recommended).
    pub fn builder() -> MyInstrumentBuilder {
        MyInstrumentBuilder::default()
    }
}
```

### 3. Implement the Instrument Trait

```rust
use crate::pricer::InstrumentType;
use crate::results::ValuationResult;
use crate::metrics::MetricId;
use finstack_core::market_data::context::MarketContext;
use std::any::Any;

impl Instrument for MyInstrument {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> InstrumentType {
        InstrumentType::MyInstrument
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        // Implement fast NPV calculation
        let pricer = MyInstrumentPricer;
        pricer.price(self, market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        // Calculate NPV + requested metrics
        let pricer = MyInstrumentPricer;
        pricer.price_with_metrics(self, market, as_of, metrics)
    }

    fn market_dependencies(&self) -> MarketDependencies {
        MarketDependencies::from_curve_dependencies(self)
    }
}
```

### 4. Implement the Pricer

**Option A: Use GenericInstrumentPricer (Recommended for most instruments)**

If your instrument's pricing logic is fully contained in its `value()` method, use `GenericInstrumentPricer` to eliminate boilerplate:

```rust
// In your registration (pricer/registry.rs), no separate pricer.rs needed:
use crate::instruments::common::pricing::GenericInstrumentPricer;
use crate::instruments::my_instrument::MyInstrument;

// Register the generic pricer
registry.register(
    Arc::new(GenericInstrumentPricer::<MyInstrument>::discounting(InstrumentType::MyInstrument))
);
```

This approach:
- Delegates to `MyInstrument::value()` automatically
- Handles type-safe downcasting
- Returns properly stamped `ValuationResult`
- Requires no separate `pricer.rs` file

For instruments using specialized models (e.g., hazard rates for CDS):

```rust
registry.register(
    Arc::new(GenericInstrumentPricer::<MyInstrument>::new(
        InstrumentType::MyInstrument,
        ModelKey::HazardRate
    ))
);
```

**Option B: Custom Pricer (For complex pricing logic)**

If you need custom pricer logic beyond the instrument's `value()` method:

```rust
// pricer.rs
use super::MyInstrument;
use crate::pricer::Pricer;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

pub struct MyInstrumentPricer;

impl Pricer<MyInstrument> for MyInstrumentPricer {
    fn price(
        &self,
        instrument: &MyInstrument,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get market data
        let curve = market.get_discount_curve(&instrument.discount_curve_id)?;

        // Calculate discount factor
        let df = curve.discount_factor(as_of, instrument.maturity)?;

        // Simple present value calculation
        let pv = instrument.notional * df;

        Ok(pv)
    }
}
```

**When to use each approach:**
- Use `GenericInstrumentPricer` (Option A) for most instruments where pricing logic is in `value()`
- Use a custom pricer (Option B) when you need:
  - Multiple pricing models with different registration
  - Specialized caching or optimization
  - Complex model switching logic

### 5. Add Metrics (`metrics/mod.rs`)

```rust
//! Risk metrics for MyInstrument.

mod my_metric;

pub use my_metric::MyMetric;

use crate::metrics::{MetricCalculator, MetricId};
use crate::instruments::my_instrument::MyInstrument;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Calculate a specific metric for MyInstrument.
pub fn calculate_my_metric(
    instrument: &MyInstrument,
    market: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    // Metric calculation logic
    Ok(0.0)
}
```

### 6. Create Module Entry Point (`mod.rs`)

```rust
//! My instrument module documentation.
//!
//! Describe what this instrument is, how it's priced, key formulas, etc.

pub mod pricer;
pub mod metrics;
mod types;

pub use types::MyInstrument;
pub use pricer::MyInstrumentPricer;
```

### 7. Register in Parent Module (`instruments/mod.rs`)

Add to `mod.rs`:

```rust
/// my instrument module.
pub mod my_instrument;

// In exports section:
pub use my_instrument::MyInstrument;
```

### 8. Add to InstrumentType Enum

In `finstack/valuations/src/pricer/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentType {
    // ... existing types
    MyInstrument,
}
```

### 9. Add to JSON Loader (`json_loader.rs`)

Add variant to `InstrumentJson` enum:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "spec", rename_all = "snake_case")]
pub enum InstrumentJson {
    // ... existing variants
    MyInstrument(MyInstrument),
}
```

Add to `into_boxed()` method:

```rust
impl InstrumentJson {
    pub fn into_boxed(self) -> Result<Box<dyn Instrument>> {
        match self {
            // ... existing matches
            InstrumentJson::MyInstrument(i) => Ok(Box::new(i)),
        }
    }
}
```

Add to manual `Deserialize` implementation (find the match on `ty.as_str()`):

```rust
"my_instrument" => serde_json::from_str(&spec_str)
    .map(Self::MyInstrument)
    .map_err(D::Error::custom),
```

And add `"my_instrument"` to the error list at the bottom.

### 10. Register Pricer in Engine

In `finstack/valuations/src/pricer/registry.rs`:

```rust
use crate::instruments::my_instrument::MyInstrumentPricer;

// In PricerRegistry::default():
registry.register(InstrumentType::MyInstrument, Arc::new(MyInstrumentPricer));
```

### 11. Write Tests

Add tests in `mod.rs` or separate `tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use time::macros::date;

    #[test]
    fn test_my_instrument_creation() {
        let instrument = MyInstrument::new(
            "MY-001",
            Money::new(1_000_000.0, Currency::USD),
            date!(2030-01-01),
            "USD-OIS",
        );

        assert_eq!(instrument.id(), "MY-001");
    }

    #[test]
    fn test_json_roundtrip() {
        let instrument = MyInstrument::new(/* ... */);
        let json = InstrumentJson::MyInstrument(instrument.clone());
        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: InstrumentJson = serde_json::from_str(&serialized).unwrap();

        // Verify deserialization
    }
}
```

### 12. Add Documentation

Add comprehensive rustdoc:

```rust
//! My instrument module.
//!
//! # Overview
//!
//! Description of the instrument type...
//!
//! # Pricing
//!
//! Mathematical formulas and methodology...
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::instruments::MyInstrument;
//! // Usage example...
//! ```
//!
//! # See Also
//!
//! - Related instruments
//! - Relevant models
```

### Checklist

- [ ] Created module directory with `mod.rs`, `types.rs`, `pricer.rs`
- [ ] Defined struct with `#[serde(deny_unknown_fields)]`
- [ ] Implemented `Instrument` trait
- [ ] Implemented `Pricer<T>` trait
- [ ] Added to `InstrumentType` enum
- [ ] Added to `InstrumentJson` enum (3 places: variant, into_boxed, Deserialize)
- [ ] Registered pricer in `PricerRegistry`
- [ ] Added metrics (if applicable)
- [ ] Added tests (construction, pricing, JSON roundtrip)
- [ ] Added comprehensive documentation
- [ ] Ran `mise run all-lint` and `mise run rust-test`

## Design Principles

1. **Currency Safety**: All `Money` types carry explicit currency; no implicit conversions
2. **Determinism**: Use `Decimal` for pricing (via global config); parallel = serial
3. **Serde Stability**: All public types use `deny_unknown_fields` for forward compatibility
4. **Builder Pattern**: Complex instruments use builders for ergonomic construction
5. **Lazy Metrics**: Metrics computed on-demand via `price_with_metrics()`
6. **Type Safety**: Strong typing (newtype IDs, enums) prevents accidental mismatches
7. **Documentation**: Every public type/function has comprehensive rustdoc
8. **Testing**: Unit, integration, and golden tests for all instruments

## Performance Considerations

- **Fast Path**: `value()` is optimized for NPV-only calculations (hot path for portfolio aggregation)
- **Metrics on Demand**: Only requested metrics are computed in `price_with_metrics()`
- **Parallel Pricing**: Portfolio-level parallelism via Rayon (maintains determinism)
- **Curve Caching**: Market context caches interpolated curve values
- **Decimal Mode**: Configurable (Decimal for correctness, f64 for performance benchmarks)

## Related Modules

- **[`cashflow`](../cashflow/)**: Cashflow generation and leg construction
- **[`pricer`](../pricer/)**: Pricer trait and registry
- **[`metrics`](../metrics/)**: Metric IDs and calculator infrastructure
- **[`results`](../results/)**: ValuationResult and period PV aggregation
- **[`calibration`](../calibration/)**: Curve bootstrapping and calibration

## References

- [Finstack Core Types](../../core/)
- [Market Data](../../core/market_data/)
- [Pricing Models Documentation](./common/models/)
- [Monte Carlo Engine](./common/mc/)
- [Cashflow Builder](../cashflow/)

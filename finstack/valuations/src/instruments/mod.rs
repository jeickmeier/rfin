//! Financial instruments for pricing, risk, and cashflow analysis.
//!
//! This module provides 50+ instrument types across fixed income, rates, credit,
//! equity, FX, commodities, and exotic options. All instruments implement the
//! [`Instrument`] trait for unified pricing and risk metric computation.
//!
//! # Organization
//!
//! Instruments are organized by asset class:
//!
//! - [`fixed_income`]: Bonds, loans, MBS, CMOs, structured credit
//! - [`rates`]: Swaps, caps/floors, swaptions, deposits, repos
//! - [`credit_derivatives`]: CDS, indices, tranches, options
//! - [`equity`]: Options, variance swaps, TRS, DCF, private markets
//! - [`fx`]: Spots, forwards, swaps, options, barriers, quantos
//! - [`commodity`]: Forwards, swaps, options
//! - [`exotics`]: Asian, barrier, lookback, basket options
//!
//! # Core Trait
//!
//! All instruments implement [`Instrument`], providing:
//! - `id()`: Unique instrument identifier
//! - `key()`: Type classification for pricer dispatch
//! - `value()`: Fast NPV calculation
//! - `price_with_metrics()`: NPV plus risk metrics (DV01, Greeks, etc.)
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::instruments::common::traits::Instrument;
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::create_date;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let issue = create_date(2025, Month::January, 15)?;
//! let maturity = create_date(2030, Month::January, 15)?;
//!
//! let bond = Bond::fixed(
//!     "US-TREASURY-5Y",
//!     Money::new(1_000_000.0, Currency::USD),
//!     0.045, // 4.5% coupon
//!     issue,
//!     maturity,
//!     "USD-OIS",
//! )?;
//!
//! // Access via Instrument trait
//! assert_eq!(bond.id(), "US-TREASURY-5Y");
//! # Ok(())
//! # }
//! ```
//!
//! # API Layers
//!
//! - **Layer 1 (ergonomic)**: `finstack_valuations::instruments::*` for instrument types and
//!   convenience re-exports.
//! - **Layer 2 (canonical shared API)**: `finstack_valuations::instruments::common` for shared
//!   traits, parameters, schedules, models, and Monte Carlo primitives.
//! - **Internal**: `common_impl` is crate-private plumbing.
//!
//! ## Migration
//!
//! ```rust,ignore
//! // Before
//! use finstack_valuations::instruments::{FixedLegSpec, FloatLegSpec, Instrument};
//!
//! // After
//! use finstack_valuations::instruments::common::{
//!     parameters::*,
//!     traits::Instrument,
//! };
//! ```
//!
//! # Supported Instrument Types
//!
//! ## Fixed Income
//! | Type | Description |
//! |------|-------------|
//! | [`Bond`] | Fixed/floating-rate bonds with embedded options |
//! | [`InflationLinkedBond`] | TIPS, index-linked gilts |
//! | [`ConvertibleBond`] | Bonds with equity conversion |
//! | [`TermLoan`] | Bilateral term loans |
//! | [`RevolvingCredit`] | Revolving credit facilities |
//! | [`StructuredCredit`] | ABS, CLO, RMBS, CMBS |
//! | [`AgencyMbsPassthrough`] | Agency MBS pass-throughs |
//! | [`AgencyCmo`] | Collateralized mortgage obligations |
//!
//! ## Interest Rates
//! | Type | Description |
//! |------|-------------|
//! | [`InterestRateSwap`] | Plain vanilla IRS |
//! | [`BasisSwap`] | Floating-for-floating swaps |
//! | [`Swaption`] | Options on swaps |
//! | [`InterestRateOption`] | Caps, floors, collars |
//! | [`Deposit`] | Money market deposits |
//! | [`Repo`] | Repurchase agreements |
//!
//! ## Credit Derivatives
//! | Type | Description |
//! |------|-------------|
//! | [`CreditDefaultSwap`] | Single-name CDS |
//! | [`CDSIndex`] | Credit indices (CDX, iTraxx) |
//! | [`CDSTranche`] | Synthetic CDO tranches |
//! | [`CDSOption`] | Options on CDS spreads |
//!
//! ## Equity & FX
//! | Type | Description |
//! |------|-------------|
//! | [`EquityOption`] | Vanilla equity options |
//! | [`FxOption`] | FX options (Garman-Kohlhagen) |
//! | [`VarianceSwap`] | Variance/volatility swaps |
//! | [`FxSwap`] | FX forwards and swaps |
//!
//! # See Also
//!
//! - [`Instrument`] for the core instrument trait
//! - [`Attributes`] for tagging and scenario selection
//! - [`crate::pricer`] for pricing registry and dispatch
//! - [`crate::metrics`] for risk metric calculations

// Common functionality (traits, macros, models, helpers)
#[macro_use]
#[path = "common/mod.rs"]
pub(crate) mod common_impl;

/// Shared functionality used across multiple instruments.
///
/// This module groups reusable building blocks (traits, parameters, schedules, models, etc.)
/// behind a single stable namespace: `finstack_valuations::instruments::common`.
/// Prefer this module for shared parameters/traits and advanced model access.
pub mod common {
    pub use super::common_impl::dependencies::{FxPair, MarketDependencies};
    pub use super::common_impl::discountable::Discountable;
    pub use super::common_impl::fx_dates::{
        add_joint_business_days, adjust_joint_calendar, roll_spot_date, ResolvedCalendarPair,
    };
    pub use super::common_impl::helpers::validate_currency_consistency;
    pub use super::common_impl::period_pv::PeriodizedPvExt;
    pub use super::common_impl::traits::{
        Attributes, CurveDependencies, CurveIdVec, EquityDependencies, EquityInstrumentDeps,
        Instrument, InstrumentCurves, PricingOptions,
    };
    pub use finstack_core::dates::fx::resolve_calendar;

    /// Market dependency types (curves, FX pairs, etc.).
    pub mod dependencies {
        pub use super::super::common_impl::dependencies::*;
    }

    /// Discounting/NPV helper traits and adapters.
    pub mod discountable {
        pub use super::super::common_impl::discountable::*;
    }

    /// FX date and joint-calendar helpers.
    pub mod fx_dates {
        pub use super::super::common_impl::fx_dates::*;
    }

    /// Shared helper functions used across instruments.
    pub mod helpers {
        pub use super::super::common_impl::helpers::*;
    }

    /// Shared parameter types (legs, schedules, market params, conventions).
    pub mod parameters {
        pub use super::super::common_impl::parameters::*;
    }

    /// Periodized PV helpers (per-period contributions, aggregation).
    pub mod period_pv {
        pub use super::super::common_impl::period_pv::*;
    }

    /// Shared pricing infrastructure (schedules, generic pricers, TRS engine, etc.).
    pub mod pricing {
        pub use super::super::common_impl::pricing::*;
    }

    /// Core instrument traits and metadata (`Instrument`, `Attributes`, dependencies).
    pub mod traits {
        pub use super::super::common_impl::traits::*;
    }

    /// Pricing models (closed-form, trees, volatility, etc.).
    pub mod models {
        pub use super::super::common_impl::models::*;
    }

    /// Monte Carlo primitives and engines (requires `mc` feature).
    #[cfg(feature = "mc")]
    pub mod mc {
        pub use super::super::common_impl::mc::*;
    }
}

// === Category Modules ===
/// Commodity derivatives.
pub mod commodity;
/// Credit derivatives: CDS and related instruments.
pub mod credit_derivatives;
/// Equity instruments and equity derivatives.
pub mod equity;
/// Exotic and path-dependent options.
pub mod exotics;
/// Fixed income instruments: bonds, loans, MBS, and structured products.
pub mod fixed_income;
/// FX instruments and FX derivatives.
pub mod fx;
/// Interest rate derivatives and money market instruments.
pub mod rates;

// === Core Instrument Types ===
// Fixed Income
pub use fixed_income::{
    AgencyCmo, AgencyMbsPassthrough, AgencyProgram, AgencyTba, Bond, BondFuture, BondFutureBuilder,
    BondFutureSpecs, CmoTranche, CmoTrancheType, CmoWaterfall, ConvertibleBond, DeliverableBond,
    DollarRoll, FIIndexTotalReturnSwap, InflationLinkedBond, PoolType, RevolvingCredit,
    StructuredCredit, TbaTerm, TermLoan,
};

// Rates
pub use rates::{
    BasisSwap, CmsOption, CollateralSpec, CollateralType, Deposit, ForwardRateAgreement,
    InflationCapFloor, InflationCapFloorType, InflationSwap, InterestRateFuture,
    InterestRateOption, InterestRateSwap, RangeAccrual, RateOptionType, Repo, RepoType, Swaption,
    XccySwap, YoYInflationSwap,
};

// Credit Derivatives
pub use credit_derivatives::{CDSIndex, CDSOption, CDSTranche, CreditDefaultSwap};

// Equity
pub use equity::{
    Autocallable, CliquetOption, DiscountedCashFlow, Equity, EquityFutureSpecs, EquityIndexFuture,
    EquityOption, EquityTotalReturnSwap, FinalPayoffType, PrivateMarketsFund, RealEstateAsset,
    RealEstateValuationMethod, TerminalValueSpec, VarianceSwap, VolIndexContractSpecs,
    VolIndexOptionSpecs, VolatilityIndexFuture, VolatilityIndexOption,
};

// FX
pub use fx::FxVarianceSwap;
pub use fx::{
    BarrierDirection, DigitalPayoutType, FxDigitalOption, FxTouchOption, PayoutTiming, TouchType,
};
pub use fx::{FxBarrierOption, FxForward, FxOption, FxSpot, FxSwap, Ndf, QuantoOption};

// Commodity
pub use commodity::{CommodityAsianOption, CommodityForward, CommodityOption, CommoditySwap};

// Exotics
pub use exotics::{
    AsianOption, AveragingMethod, BarrierOption, BarrierType, Basket, LookbackOption, LookbackType,
};

// === Common Functionality ===
pub use common_impl::dependencies::{FxPair, MarketDependencies};
pub use common_impl::discountable::Discountable;
pub use common_impl::period_pv::PeriodizedPvExt;
pub use common_impl::pricing::{TotalReturnLegParams, TrsEngine, TrsReturnModel};
pub use common_impl::traits::{
    Attributes, CurveDependencies, CurveIdVec, EquityDependencies, EquityInstrumentDeps,
    Instrument, InstrumentCurves, PricingOptions,
};

// === Parameter Types ===
pub use common_impl::parameters::{
    BasisSwapLeg, BondConvention, ContractSpec, CreditParams, EquityOptionParams,
    EquityUnderlyingParams, ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec,
    FxOptionParams, FxUnderlyingParams, IRSConvention, IndexUnderlyingParams,
    InterestRateOptionParams, OptionMarketParams, OptionType, ParRateMethod, PayReceive,
    PremiumLegSpec, ProtectionLegSpec, ScheduleSpec, SettlementType, TotalReturnLegSpec,
    UnderlyingParams,
};

// Re-export TRS common types
pub use common_impl::parameters::trs_common::{TrsScheduleSpec, TrsSide};

/// Market parameter types for backward compatibility with tests.
///
/// This module re-exports commonly used market parameter types.
pub mod market {
    pub use super::common_impl::parameters::market::{ExerciseStyle, OptionType, SettlementType};
}

/// Leg parameter types for backward compatibility.
pub mod legs {
    pub use super::common_impl::parameters::legs::PayReceive;
}

/// Pricing overrides module.
pub mod pricing_overrides;
pub use pricing_overrides::PricingOverrides;

// === JSON Import/Export ===
pub mod json_loader;

pub use json_loader::{InstrumentEnvelope, InstrumentJson};

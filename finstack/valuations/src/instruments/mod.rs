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
//! use finstack_valuations::instruments::{Bond, Instrument};
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
//! | [`CdsTranche`] | Synthetic CDO tranches |
//! | [`CdsOption`] | Options on CDS spreads |
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
//! - [`common::traits::Instrument`] for the core instrument trait
//! - [`common::traits::Attributes`] for tagging and scenario selection
//! - [`crate::pricer`] for pricing registry and dispatch
//! - [`crate::metrics`] for risk metric calculations

// Common functionality (traits, macros, models, helpers)
#[macro_use]
#[doc(hidden)]
pub mod common;

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

// Legacy module aliases retained for internal compatibility only.

// Fixed Income
pub(crate) use fixed_income::bond;
pub(crate) use fixed_income::bond_future;
pub(crate) use fixed_income::convertible;
pub(crate) use fixed_income::dollar_roll;
pub(crate) use fixed_income::fi_trs;
pub(crate) use fixed_income::inflation_linked_bond;
pub(crate) use fixed_income::revolving_credit;
pub(crate) use fixed_income::structured_credit;
pub(crate) use fixed_income::term_loan;

// Rates
pub(crate) use rates::basis_swap;
pub(crate) use rates::cap_floor;
pub(crate) use rates::cms_option;
pub(crate) use rates::deposit;
pub(crate) use rates::fra;
pub(crate) use rates::inflation_cap_floor;
pub(crate) use rates::inflation_swap;
pub(crate) use rates::ir_future;
pub(crate) use rates::irs;
pub(crate) use rates::range_accrual;
pub(crate) use rates::repo;
pub(crate) use rates::swaption;
pub(crate) use rates::xccy_swap;

// Credit Derivatives
pub(crate) use credit_derivatives::cds;
pub(crate) use credit_derivatives::cds_index;
pub(crate) use credit_derivatives::cds_option;
pub(crate) use credit_derivatives::cds_tranche;

// Equity
pub(crate) use equity::autocallable;
pub(crate) use equity::cliquet_option;
pub(crate) use equity::equity_index_future;
pub(crate) use equity::equity_option;
pub(crate) use equity::equity_trs;
pub(crate) use equity::real_estate;
pub(crate) use equity::variance_swap;
pub(crate) use equity::vol_index_future;
pub(crate) use equity::vol_index_option;

// FX
pub(crate) use fx::fx_barrier_option;
pub(crate) use fx::fx_forward;
pub(crate) use fx::fx_option;
pub(crate) use fx::fx_spot;
pub(crate) use fx::fx_swap;
pub(crate) use fx::fx_variance_swap;
pub(crate) use fx::ndf;
pub(crate) use fx::quanto_option;

// Commodity
pub(crate) use commodity::commodity_forward;
pub(crate) use commodity::commodity_option;
pub(crate) use commodity::commodity_swap;

// Exotics
pub(crate) use exotics::asian_option;
pub(crate) use exotics::barrier_option;
pub(crate) use exotics::basket;
pub(crate) use exotics::lookback_option;

pub(crate) use equity::dcf_equity as dcf;
pub(crate) use equity::pe_fund as private_markets_fund;
pub(crate) use fixed_income::cmo as agency_cmo;
pub(crate) use fixed_income::mbs_passthrough as agency_mbs_passthrough;
pub(crate) use fixed_income::tba as agency_tba;

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
pub use credit_derivatives::{CDSIndex, CdsOption, CdsTranche, CreditDefaultSwap};

// Equity
pub use equity::{
    Autocallable, CliquetOption, DiscountedCashFlow, Equity, EquityFutureSpecs, EquityIndexFuture,
    EquityOption, EquityTotalReturnSwap, FinalPayoffType, PrivateMarketsFund, RealEstateAsset,
    RealEstateValuationMethod, TerminalValueSpec, VarianceSwap, VolIndexContractSpecs,
    VolIndexOptionSpecs, VolatilityIndexFuture, VolatilityIndexOption,
};

// FX
pub use fx::FxVarianceSwap;
pub use fx::{FxBarrierOption, FxForward, FxOption, FxSpot, FxSwap, Ndf, QuantoOption};

// Commodity
pub use commodity::{CommodityForward, CommodityOption, CommoditySwap};

// Exotics
pub use exotics::{
    AsianOption, AveragingMethod, BarrierOption, BarrierType, Basket, LookbackOption, LookbackType,
};

// === Common Functionality ===
pub use common::period_pv::PeriodizedPvExt;
pub use common::traits::{
    Attributes, CurveDependencies, CurveIdVec, EquityDependencies, EquityInstrumentDeps,
    Instrument, InstrumentCurves, PricingOptions,
};

// === Parameter Types ===
pub use common::parameters::{
    BasisSwapLeg, BondConvention, ContractSpec, CreditParams, EquityOptionParams,
    EquityUnderlyingParams, ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec,
    FxOptionParams, FxUnderlyingParams, IRSConvention, IndexUnderlyingParams,
    InterestRateOptionParams, OptionMarketParams, OptionType, ParRateMethod, PayReceive,
    PremiumLegSpec, ProtectionLegSpec, ScheduleSpec, SettlementType, TotalReturnLegSpec,
    UnderlyingParams,
};

// Re-export TRS common types
pub use common::parameters::trs_common::{TrsScheduleSpec, TrsSide};

/// Market parameter types for backward compatibility with tests.
///
/// This module re-exports commonly used market parameter types.
pub mod market {
    pub use super::common::parameters::market::{ExerciseStyle, OptionType, SettlementType};
}

/// Leg parameter types for backward compatibility.
pub mod legs {
    pub use super::common::parameters::legs::PayReceive;
}

/// Pricing overrides module.
pub mod pricing_overrides;
pub use pricing_overrides::PricingOverrides;

// === JSON Import/Export ===
#[cfg(feature = "serde")]
pub mod json_loader;

#[cfg(feature = "serde")]
pub use json_loader::{InstrumentEnvelope, InstrumentJson};

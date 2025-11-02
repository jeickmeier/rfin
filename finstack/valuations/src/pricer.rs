//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.

use crate::instruments::common::traits::Instrument as Priceable;
use finstack_core::market_data::MarketContext as Market;

// ========================= KEYS =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
/// Strongly-typed instrument classification for pricer dispatch.
///
/// Each variant represents a distinct instrument type with its own pricing
/// logic and risk characteristics. Used by the pricing registry to route
/// instruments to appropriate pricer implementations.
pub enum InstrumentType {
    /// Fixed or floating-rate bond (plain vanilla, callable, amortizing).
    Bond = 1,
    /// Term loan or bilateral lending facility.
    Loan = 2,
    /// Credit Default Swap (single-name credit protection).
    CDS = 3,
    /// CDS Index (portfolio credit protection, e.g., CDX, iTraxx).
    CDSIndex = 4,
    /// CDS Tranche (mezzanine credit exposure via synthetic CDO).
    CDSTranche = 5,
    /// Option on Credit Default Swap.
    CDSOption = 6,
    /// Interest Rate Swap (fixed-for-floating exchange).
    IRS = 7,
    /// Interest rate cap or floor (portfolio of caplets/floorlets).
    CapFloor = 8,
    /// Option on interest rate swap.
    Swaption = 9,
    /// Total Return Swap (equity or fixed income).
    TRS = 10,
    /// Basis swap (floating-for-floating with spread).
    BasisSwap = 11,
    /// Multi-asset basket (worst-of, best-of, or weighted).
    Basket = 12,
    /// Convertible bond (bond with embedded equity option).
    Convertible = 13,
    /// Money market deposit (single-period).
    Deposit = 14,
    /// Vanilla equity option (call or put on single stock or index).
    EquityOption = 15,
    /// FX option (Garman-Kohlhagen model).
    FxOption = 16,
    /// FX spot position (currency pair).
    FxSpot = 17,
    /// FX forward or FX swap.
    FxSwap = 18,
    /// Inflation-linked bond (TIPS, index-linked gilts).
    InflationLinkedBond = 19,
    /// Zero-coupon inflation swap.
    InflationSwap = 20,
    /// Interest rate futures contract.
    InterestRateFuture = 21,
    /// Variance swap (volatility exposure).
    VarianceSwap = 22,
    /// Equity spot position (shares in single stock or fund).
    Equity = 23,
    /// Repurchase agreement (repo or reverse repo).
    Repo = 24,
    /// Forward Rate Agreement (forward-starting interest rate contract).
    FRA = 25,
    /// Structured credit (ABS, RMBS, CMBS, CLO with tranches and waterfall).
    StructuredCredit = 26,
    /// Private markets fund (PE/credit fund with waterfall).
    PrivateMarketsFund = 30,
    /// Revolving credit facility with drawdown/repayment.
    RevolvingCredit = 31,
    /// Asian option (payoff based on average price).
    AsianOption = 32,
    /// Barrier option (knock-in or knock-out on barrier cross).
    BarrierOption = 33,
    /// Lookback option (payoff based on path extremum).
    LookbackOption = 34,
    /// Quanto option (cross-currency option with quanto adjustment).
    QuantoOption = 35,
    /// Autocallable structured note (early redemption feature).
    Autocallable = 36,
    /// CMS option (constant maturity swap option).
    CmsOption = 37,
    /// Cliquet option (ratchet option with periodic resets).
    CliquetOption = 38,
    /// Range accrual note (coupon accrues when rate in range).
    RangeAccrual = 39,
    /// FX barrier option (FX option with knock-in/out barrier).
    FxBarrierOption = 40,
    /// Term loan (optionally Delayed Draw Term Loan)
    TermLoan = 41,
}

impl InstrumentType {
    /// Returns the canonical string representation for metrics registry lookups.
    ///
    /// This format matches the instrument type tags used in the metrics registry
    /// and is TitleCase (e.g., "Bond", "InterestRateSwap").
    pub fn as_str(&self) -> &'static str {
        match self {
            InstrumentType::Bond => "Bond",
            InstrumentType::Loan => "Loan",
            InstrumentType::CDS => "CDS",
            InstrumentType::CDSIndex => "CDSIndex",
            InstrumentType::CDSTranche => "CDSTranche",
            InstrumentType::CDSOption => "CDSOption",
            InstrumentType::IRS => "InterestRateSwap",
            InstrumentType::CapFloor => "InterestRateOption",
            InstrumentType::Swaption => "Swaption",
            InstrumentType::TRS => "TRS",
            InstrumentType::BasisSwap => "BasisSwap",
            InstrumentType::Basket => "Basket",
            InstrumentType::Convertible => "ConvertibleBond",
            InstrumentType::Deposit => "Deposit",
            InstrumentType::EquityOption => "EquityOption",
            InstrumentType::FxOption => "FxOption",
            InstrumentType::FxSpot => "FxSpot",
            InstrumentType::FxSwap => "FxSwap",
            InstrumentType::InflationLinkedBond => "InflationLinkedBond",
            InstrumentType::InflationSwap => "InflationSwap",
            InstrumentType::InterestRateFuture => "InterestRateFuture",
            InstrumentType::VarianceSwap => "VarianceSwap",
            InstrumentType::Equity => "Equity",
            InstrumentType::Repo => "Repo",
            InstrumentType::FRA => "FRA",
            InstrumentType::StructuredCredit => "StructuredCredit",
            InstrumentType::PrivateMarketsFund => "PrivateMarketsFund",
            InstrumentType::RevolvingCredit => "RevolvingCredit",
            InstrumentType::AsianOption => "AsianOption",
            InstrumentType::BarrierOption => "BarrierOption",
            InstrumentType::LookbackOption => "LookbackOption",
            InstrumentType::QuantoOption => "QuantoOption",
            InstrumentType::Autocallable => "Autocallable",
            InstrumentType::CmsOption => "CmsOption",
            InstrumentType::CliquetOption => "CliquetOption",
            InstrumentType::RangeAccrual => "RangeAccrual",
            InstrumentType::FxBarrierOption => "FxBarrierOption",
            InstrumentType::TermLoan => "TermLoan",
        }
    }
}

impl std::fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            InstrumentType::Bond => "bond",
            InstrumentType::Loan => "loan",
            InstrumentType::CDS => "cds",
            InstrumentType::CDSIndex => "cds_index",
            InstrumentType::CDSTranche => "cds_tranche",
            InstrumentType::CDSOption => "cds_option",
            InstrumentType::IRS => "irs",
            InstrumentType::CapFloor => "cap_floor",
            InstrumentType::Swaption => "swaption",
            InstrumentType::TRS => "trs",
            InstrumentType::BasisSwap => "basis_swap",
            InstrumentType::Basket => "basket",
            InstrumentType::Convertible => "convertible",
            InstrumentType::Deposit => "deposit",
            InstrumentType::EquityOption => "equity_option",
            InstrumentType::FxOption => "fx_option",
            InstrumentType::FxSpot => "fx_spot",
            InstrumentType::FxSwap => "fx_swap",
            InstrumentType::InflationLinkedBond => "inflation_linked_bond",
            InstrumentType::InflationSwap => "inflation_swap",
            InstrumentType::InterestRateFuture => "interest_rate_future",
            InstrumentType::VarianceSwap => "variance_swap",
            InstrumentType::Equity => "equity",
            InstrumentType::Repo => "repo",
            InstrumentType::FRA => "fra",
            InstrumentType::StructuredCredit => "structured_credit",
            InstrumentType::PrivateMarketsFund => "private_markets_fund",
            InstrumentType::RevolvingCredit => "revolving_credit",
            InstrumentType::AsianOption => "asian_option",
            InstrumentType::BarrierOption => "barrier_option",
            InstrumentType::LookbackOption => "lookback_option",
            InstrumentType::QuantoOption => "quanto_option",
            InstrumentType::Autocallable => "autocallable",
            InstrumentType::CmsOption => "cms_option",
            InstrumentType::CliquetOption => "cliquet_option",
            InstrumentType::RangeAccrual => "range_accrual",
            InstrumentType::FxBarrierOption => "fx_barrier_option",
            InstrumentType::TermLoan => "term_loan",
        };
        write!(f, "{}", label)
    }
}

impl std::str::FromStr for InstrumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "bond" => Ok(InstrumentType::Bond),
            "loan" => Ok(InstrumentType::Loan),
            "cds" => Ok(InstrumentType::CDS),
            "cds_index" | "cdsindex" => Ok(InstrumentType::CDSIndex),
            "cds_tranche" | "cdstranche" => Ok(InstrumentType::CDSTranche),
            "cds_option" | "cdsoption" => Ok(InstrumentType::CDSOption),
            "irs" | "swap" | "interest_rate_swap" => Ok(InstrumentType::IRS),
            "cap_floor" | "capfloor" | "interest_rate_option" => Ok(InstrumentType::CapFloor),
            "swaption" => Ok(InstrumentType::Swaption),
            "trs" | "total_return_swap" => Ok(InstrumentType::TRS),
            "basis_swap" | "basisswap" => Ok(InstrumentType::BasisSwap),
            "basket" => Ok(InstrumentType::Basket),
            "convertible" | "convertible_bond" => Ok(InstrumentType::Convertible),
            "deposit" => Ok(InstrumentType::Deposit),
            "equity_option" | "equityoption" => Ok(InstrumentType::EquityOption),
            "fx_option" | "fxoption" => Ok(InstrumentType::FxOption),
            "fx_spot" | "fxspot" => Ok(InstrumentType::FxSpot),
            "fx_swap" | "fxswap" => Ok(InstrumentType::FxSwap),
            "inflation_linked_bond" | "ilb" => Ok(InstrumentType::InflationLinkedBond),
            "inflation_swap" => Ok(InstrumentType::InflationSwap),
            "interest_rate_future" | "ir_future" | "irfuture" => {
                Ok(InstrumentType::InterestRateFuture)
            }
            "variance_swap" | "varianceswap" => Ok(InstrumentType::VarianceSwap),
            "equity" => Ok(InstrumentType::Equity),
            "repo" => Ok(InstrumentType::Repo),
            "fra" => Ok(InstrumentType::FRA),
            // Unified structured credit type (accepts legacy names for backward compatibility)
            "structured_credit" | "clo" | "abs" | "rmbs" | "cmbs" => {
                Ok(InstrumentType::StructuredCredit)
            }
            "private_markets_fund" | "pmf" => Ok(InstrumentType::PrivateMarketsFund),
            "revolving_credit" | "revolver" | "rc" => Ok(InstrumentType::RevolvingCredit),
            "asian_option" | "asian" => Ok(InstrumentType::AsianOption),
            "barrier_option" | "barrier" => Ok(InstrumentType::BarrierOption),
            "lookback_option" | "lookback" => Ok(InstrumentType::LookbackOption),
            "quanto_option" | "quanto" => Ok(InstrumentType::QuantoOption),
            "autocallable" | "auto_callable" => Ok(InstrumentType::Autocallable),
            "cms_option" | "cms" => Ok(InstrumentType::CmsOption),
            "cliquet_option" | "cliquet" => Ok(InstrumentType::CliquetOption),
            "range_accrual" | "range_accrual_note" => Ok(InstrumentType::RangeAccrual),
            "fx_barrier_option" | "fx_barrier" => Ok(InstrumentType::FxBarrierOption),
            "term_loan" | "termloan" | "loan_term" => Ok(InstrumentType::TermLoan),
            other => Err(format!("Unknown instrument type: {}", other)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
/// Model Key enumeration.
pub enum ModelKey {
    /// Discounting variant.
    Discounting = 1,
    /// Tree variant.
    Tree = 2,
    /// Black76 variant.
    Black76 = 3,
    /// Hull White1 F variant.
    HullWhite1F = 4,
    /// Hazard Rate variant.
    HazardRate = 5,
    /// Monte Carlo with GBM process
    MonteCarloGBM = 10,
    /// Monte Carlo with Heston stochastic volatility
    MonteCarloHeston = 11,
    /// Monte Carlo with Hull-White 1F (rates)
    MonteCarloHullWhite1F = 12,
    /// Barrier BS Continuous (Reiner-Rubinstein formulas)
    BarrierBSContinuous = 20,
    /// Asian Geometric BS (closed-form for geometric average)
    AsianGeometricBS = 21,
    /// Asian Turnbull-Wakeman (semi-analytical for arithmetic average)
    AsianTurnbullWakeman = 22,
    /// Lookback BS Continuous (closed-form for fixed/floating strike)
    LookbackBSContinuous = 23,
    /// Quanto BS (vanilla quanto with drift adjustment)
    QuantoBS = 24,
    /// FX Barrier BS Continuous (Reiner-Rubinstein with FX mapping)
    FxBarrierBSContinuous = 25,
    /// Heston Fourier (semi-analytical via characteristic function)
    HestonFourier = 26,
}

impl std::fmt::Display for ModelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ModelKey::Discounting => "discounting",
            ModelKey::Tree => "tree",
            ModelKey::Black76 => "black76",
            ModelKey::HullWhite1F => "hull_white_1f",
            ModelKey::HazardRate => "hazard_rate",
            ModelKey::MonteCarloGBM => "monte_carlo_gbm",
            ModelKey::MonteCarloHeston => "monte_carlo_heston",
            ModelKey::MonteCarloHullWhite1F => "monte_carlo_hull_white_1f",
            ModelKey::BarrierBSContinuous => "barrier_bs_continuous",
            ModelKey::AsianGeometricBS => "asian_geometric_bs",
            ModelKey::AsianTurnbullWakeman => "asian_turnbull_wakeman",
            ModelKey::LookbackBSContinuous => "lookback_bs_continuous",
            ModelKey::QuantoBS => "quanto_bs",
            ModelKey::FxBarrierBSContinuous => "fx_barrier_bs_continuous",
            ModelKey::HestonFourier => "heston_fourier",
        };
        write!(f, "{}", label)
    }
}

impl std::str::FromStr for ModelKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "discounting" => Ok(ModelKey::Discounting),
            "tree" | "lattice" => Ok(ModelKey::Tree),
            "black76" | "black" | "black_76" => Ok(ModelKey::Black76),
            "hull_white_1f" | "hullwhite1f" | "hw1f" => Ok(ModelKey::HullWhite1F),
            "hazard_rate" | "hazard" => Ok(ModelKey::HazardRate),
            "monte_carlo_gbm" | "mc_gbm" | "montecarlo_gbm" => Ok(ModelKey::MonteCarloGBM),
            "monte_carlo_heston" | "mc_heston" | "montecarlo_heston" => {
                Ok(ModelKey::MonteCarloHeston)
            }
            "monte_carlo_hull_white_1f" | "mc_hw1f" | "montecarlo_hw1f" => {
                Ok(ModelKey::MonteCarloHullWhite1F)
            }
            "barrier_bs_continuous" | "barrier_bs" | "barrier_continuous" => {
                Ok(ModelKey::BarrierBSContinuous)
            }
            "asian_geometric_bs" | "asian_geometric" | "geometric_asian_bs" => {
                Ok(ModelKey::AsianGeometricBS)
            }
            "asian_turnbull_wakeman" | "asian_tw" | "arithmetic_asian_tw" => {
                Ok(ModelKey::AsianTurnbullWakeman)
            }
            "lookback_bs_continuous" | "lookback_bs" | "lookback_continuous" => {
                Ok(ModelKey::LookbackBSContinuous)
            }
            "quanto_bs" | "quanto" => Ok(ModelKey::QuantoBS),
            "fx_barrier_bs_continuous" | "fx_barrier_bs" | "fx_barrier_continuous" => {
                Ok(ModelKey::FxBarrierBSContinuous)
            }
            "heston_fourier" | "heston_semi_analytical" | "heston_analytical" => {
                Ok(ModelKey::HestonFourier)
            }
            other => Err(format!("Unknown model key: {}", other)),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Pricer Key structure.
pub struct PricerKey {
    /// instrument.
    pub instrument: InstrumentType,
    /// model.
    pub model: ModelKey,
}

impl PricerKey {
    /// fn constant.
    pub const fn new(instrument: InstrumentType, model: ModelKey) -> Self {
        Self { instrument, model }
    }
}

// ========================= ERRORS =========================

/// Standardized result type for pricing operations
pub type PricingResult<T> = std::result::Result<T, PricingError>;

/// Pricing-specific errors returned by pricer implementations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PricingError {
    /// No pricer registered for the requested (instrument, model) combination.
    #[error("No pricer found for instrument={} model={}", .0.instrument, .0.model)]
    UnknownPricer(PricerKey),

    /// Instrument type mismatch during downcasting.
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch {
        /// Expected instrument type
        expected: InstrumentType,
        /// Actual instrument type
        got: InstrumentType,
    },

    /// Pricing model computation failed.
    #[error("Model failure: {0}")]
    ModelFailure(String),

    /// Invalid input parameters provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Missing market data required for pricing.
    #[error("Missing market data: {0}")]
    MissingMarketData(String),
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        PricingError::ModelFailure(err.to_string())
    }
}

impl PricingError {
    /// Create a type mismatch error with context
    pub fn type_mismatch(expected: InstrumentType, got: InstrumentType) -> Self {
        Self::TypeMismatch { expected, got }
    }

    /// Create a model failure error with context
    pub fn model_failure(msg: impl Into<String>) -> Self {
        Self::ModelFailure(msg.into())
    }

    /// Create an invalid input error with context
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a missing market data error with context
    pub fn missing_market_data(msg: impl Into<String>) -> Self {
        Self::MissingMarketData(msg.into())
    }
}

// ========================= TRAITS =========================

/// Helper function to safely downcast a trait object to a concrete instrument type.
///
/// This performs both enum-based type checking and actual type downcasting,
/// ensuring type safety at both levels.
pub fn expect_inst<T: Priceable + 'static>(
    inst: &dyn Priceable,
    expected: InstrumentType,
) -> PricingResult<&T> {
    // First check the enum-based type
    if inst.key() != expected {
        return Err(PricingError::type_mismatch(expected, inst.key()));
    }

    // Then perform actual downcast
    inst.as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| PricingError::type_mismatch(expected, inst.key()))
}

/// Trait for instrument pricers.
///
/// Each pricer handles a specific (instrument, model) combination and knows
/// how to price that instrument using the specified model.
pub trait Pricer: Send + Sync {
    /// Get the (instrument, model) key this pricer handles
    fn key(&self) -> PricerKey;

    /// Price an instrument using this pricer's model
    fn price_dyn(
        &self,
        instrument: &dyn Priceable,
        market: &Market,
        as_of: finstack_core::dates::Date,
    ) -> PricingResult<crate::results::ValuationResult>;
}

// ========================= REGISTRY =========================

use std::collections::HashMap;

/// Registry mapping (instrument type, model) pairs to pricer implementations.
///
/// Provides type-safe pricing dispatch without string comparisons or runtime
/// registration errors. Pricers are registered at compile time and looked up
/// via strongly-typed keys.
#[derive(Default)]
pub struct PricerRegistry {
    pricers: HashMap<PricerKey, Box<dyn Pricer>>,
}

impl PricerRegistry {
    /// Create a new empty pricer registry.
    ///
    /// For pre-configured registries with all standard pricers, use
    /// [`create_standard_registry()`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pricer for a specific (instrument type, model) combination.
    ///
    /// If a pricer already exists for this key, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - Pricer key identifying the (instrument type, model) pair
    /// * `pricer` - Pricer implementation for this combination
    pub fn register_pricer(&mut self, key: PricerKey, pricer: Box<dyn Pricer>) {
        self.pricers.insert(key, pricer);
    }

    /// Look up a pricer for a specific (instrument type, model) combination.
    ///
    /// # Arguments
    ///
    /// * `key` - Pricer key to look up
    ///
    /// # Returns
    ///
    /// `Some(&dyn Pricer)` if registered, `None` otherwise
    pub fn get_pricer(&self, key: PricerKey) -> Option<&dyn Pricer> {
        self.pricers.get(&key).map(|p| p.as_ref())
    }

    /// Price an instrument using the registry dispatch system.
    ///
    /// Routes the instrument to the appropriate pricer based on its type
    /// and the requested pricing model.
    ///
    /// # Arguments
    ///
    /// * `instrument` - Instrument to price (as trait object)
    /// * `model` - Pricing model to use
    /// * `market` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// `ValuationResult` with present value and metadata
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No pricer registered for this (instrument, model) combination
    /// - Pricing calculation fails
    /// - Required market data is missing
    pub fn price_with_registry(
        &self,
        instrument: &dyn Priceable,
        model: ModelKey,
        market: &Market,
        as_of: finstack_core::dates::Date,
    ) -> PricingResult<crate::results::ValuationResult> {
        let key = PricerKey::new(instrument.key(), model);
        if let Some(pricer) = self.get_pricer(key) {
            pricer.price_dyn(instrument, market, as_of)
        } else {
            Err(PricingError::UnknownPricer(key))
        }
    }
}

// ========================= REGISTRATION =========================

/// Register all standard pricers explicitly.
///
/// This function registers all instrument pricers in a single, visible location.
/// This explicit approach provides better IDE support, easier debugging, and
/// clearer dependency tracking compared to auto-registration. Registration here
/// is explicit and centralized, not implicit or macro-driven.
fn register_all_pricers(registry: &mut PricerRegistry) {
    // Bond pricers
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Discounting),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondDiscountingPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Bond, ModelKey::Tree),
        Box::new(crate::instruments::bond::pricing::pricer::SimpleBondOasPricer),
    );

    // Interest Rate Swaps
    registry.register_pricer(
        PricerKey::new(InstrumentType::IRS, ModelKey::Discounting),
        Box::new(crate::instruments::irs::pricer::SimpleIrsDiscountingPricer::default()),
    );

    // FRA
    registry.register_pricer(
        PricerKey::new(InstrumentType::FRA, ModelKey::Discounting),
        Box::new(crate::instruments::fra::pricer::SimpleFraDiscountingPricer::default()),
    );

    // Basis Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::BasisSwap, ModelKey::Discounting),
        Box::new(
            crate::instruments::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default(),
        ),
    );

    // Deposit
    registry.register_pricer(
        PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting),
        Box::new(crate::instruments::deposit::pricer::SimpleDepositDiscountingPricer::default()),
    );

    // Interest Rate Future
    registry.register_pricer(
        PricerKey::new(InstrumentType::InterestRateFuture, ModelKey::Discounting),
        Box::new(crate::instruments::ir_future::pricer::SimpleIrFutureDiscountingPricer::default()),
    );

    // Cap/Floor
    registry.register_pricer(
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76),
        Box::new(
            crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default(),
        ),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Discounting),
        Box::new(
            crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // Swaption
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::Black76),
        Box::new(crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::Discounting),
        Box::new(
            crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate),
        Box::new(crate::instruments::common::GenericInstrumentPricer::cds()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDS, ModelKey::Discounting),
        Box::new(crate::instruments::common::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting)),
    );

    // CDS Index
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS Tranche
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::HazardRate),
        Box::new(crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // CDS Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76),
        Box::new(crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // FX Spot
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSpot, ModelKey::Discounting),
        Box::new(crate::instruments::fx_spot::pricer::SimpleFxSpotDiscountingPricer),
    );

    // FX Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxSwap, ModelKey::Discounting),
        Box::new(crate::instruments::fx_swap::pricer::SimpleFxSwapDiscountingPricer::default()),
    );

    // FX Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxOption, ModelKey::Black76),
        Box::new(crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::default()),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );

    // Equity
    registry.register_pricer(
        PricerKey::new(InstrumentType::Equity, ModelKey::Discounting),
        Box::new(crate::instruments::equity::pricer::SimpleEquityDiscountingPricer),
    );

    // Equity Option
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Black76),
        Box::new(
            crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::default(),
        ),
    );
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Discounting),
        Box::new(
            crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
                ModelKey::Discounting,
            ),
        ),
    );
    // Equity Option - Heston Fourier (semi-analytical)
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::EquityOption, ModelKey::HestonFourier),
        Box::new(crate::instruments::equity_option::pricer::EquityOptionHestonFourierPricer),
    );

    // TRS (Total Return Swap)
    registry.register_pricer(
        PricerKey::new(InstrumentType::TRS, ModelKey::Discounting),
        Box::new(crate::instruments::trs::pricing::pricer::SimpleTrsDiscountingPricer),
    );

    // Convertible Bond
    registry.register_pricer(
        PricerKey::new(InstrumentType::Convertible, ModelKey::Discounting),
        Box::new(crate::instruments::convertible::pricer::SimpleConvertibleDiscountingPricer),
    );

    // Private Markets Fund
    registry.register_pricer(
        PricerKey::new(InstrumentType::PrivateMarketsFund, ModelKey::Discounting),
        Box::new(
            crate::instruments::private_markets_fund::pricer::PrivateMarketsFundDiscountingPricer,
        ),
    );

    // Inflation Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationSwap, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_swap::pricer::SimpleInflationSwapDiscountingPricer::default()),
    );

    // Inflation Linked Bond
    registry.register_pricer(
        PricerKey::new(InstrumentType::InflationLinkedBond, ModelKey::Discounting),
        Box::new(crate::instruments::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::default()),
    );

    // Variance Swap
    registry.register_pricer(
        PricerKey::new(InstrumentType::VarianceSwap, ModelKey::Discounting),
        Box::new(
            crate::instruments::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default(
            ),
        ),
    );

    // Repo
    registry.register_pricer(
        PricerKey::new(InstrumentType::Repo, ModelKey::Discounting),
        Box::new(crate::instruments::repo::pricer::SimpleRepoDiscountingPricer::default()),
    );

    // Basket
    registry.register_pricer(
        PricerKey::new(InstrumentType::Basket, ModelKey::Discounting),
        Box::new(crate::instruments::basket::SimpleBasketDiscountingPricer::default()),
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    registry.register_pricer(
        PricerKey::new(InstrumentType::StructuredCredit, ModelKey::Discounting),
        Box::new(
            crate::instruments::structured_credit::StructuredCreditDiscountingPricer::default(),
        ),
    );

    // Revolving Credit
    registry.register_pricer(
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting),
        Box::new(
            crate::instruments::revolving_credit::pricer::RevolvingCreditDiscountingPricer::new(),
        ),
    );
    // Term Loan (including DDTL)
    registry.register_pricer(
        PricerKey::new(InstrumentType::TermLoan, ModelKey::Discounting),
        Box::new(
            crate::instruments::term_loan::pricing::TermLoanDiscountingPricer::default(),
        ),
    );
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::revolving_credit::pricer::RevolvingCreditMcPricer::new()),
    );

    // Asian Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::AsianOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::asian_option::pricer::AsianOptionMcPricer::default()),
    );
    // Asian Option - Analytical (Geometric)
    registry.register_pricer(
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianGeometricBS),
        Box::new(crate::instruments::asian_option::pricer::AsianOptionAnalyticalGeometricPricer),
    );
    // Asian Option - Semi-Analytical (Turnbull-Wakeman)
    registry.register_pricer(
        PricerKey::new(InstrumentType::AsianOption, ModelKey::AsianTurnbullWakeman),
        Box::new(crate::instruments::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer),
    );

    // Barrier Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::barrier_option::pricer::BarrierOptionMcPricer::default()),
    );
    // Barrier Option - Analytical (Continuous monitoring)
    registry.register_pricer(
        PricerKey::new(InstrumentType::BarrierOption, ModelKey::BarrierBSContinuous),
        Box::new(crate::instruments::barrier_option::pricer::BarrierOptionAnalyticalPricer),
    );

    // Lookback Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::LookbackOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::lookback_option::pricer::LookbackOptionMcPricer::default()),
    );
    // Lookback Option - Analytical (Continuous monitoring)
    registry.register_pricer(
        PricerKey::new(
            InstrumentType::LookbackOption,
            ModelKey::LookbackBSContinuous,
        ),
        Box::new(crate::instruments::lookback_option::pricer::LookbackOptionAnalyticalPricer),
    );

    // Quanto Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::QuantoOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::quanto_option::pricer::QuantoOptionMcPricer::default()),
    );
    // Quanto Option - Analytical
    registry.register_pricer(
        PricerKey::new(InstrumentType::QuantoOption, ModelKey::QuantoBS),
        Box::new(crate::instruments::quanto_option::pricer::QuantoOptionAnalyticalPricer),
    );

    // Autocallable
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::Autocallable, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::autocallable::pricer::AutocallableMcPricer::default()),
    );

    // CMS Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::CmsOption, ModelKey::MonteCarloHullWhite1F),
        Box::new(crate::instruments::cms_option::pricer::CmsOptionMcPricer::new()),
    );

    // Cliquet Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::CliquetOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::cliquet_option::pricer::CliquetOptionMcPricer::default()),
    );

    // Range Accrual
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::RangeAccrual, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::range_accrual::pricer::RangeAccrualMcPricer::default()),
    );

    // FX Barrier Option
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::FxBarrierOption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::fx_barrier_option::pricer::FxBarrierOptionMcPricer::default()),
    );
    // FX Barrier Option - Analytical (Continuous monitoring)
    registry.register_pricer(
        PricerKey::new(
            InstrumentType::FxBarrierOption,
            ModelKey::FxBarrierBSContinuous,
        ),
        Box::new(crate::instruments::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer),
    );

    // Swaption LSMC (Bermudan exercise)
    #[cfg(feature = "mc")]
    registry.register_pricer(
        PricerKey::new(InstrumentType::Swaption, ModelKey::MonteCarloGBM),
        Box::new(crate::instruments::swaption::pricer::SwaptionLsmcPricer::default()),
    );
}

/// Create a standard pricer registry with all registered pricers.
///
/// This function creates a registry and explicitly registers all instrument pricers.
/// The explicit registration approach provides better visibility, IDE support, and
/// debugging capabilities compared to the previous auto-registration system.
///
/// All 40+ instrument pricers are registered in the `register_all_pricers` function.
/// Note: All pricers now use standardized parameter ordering: (instrument, market, as_of).
pub fn create_standard_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_all_pricers(&mut registry);
    registry
}

// ========================= TESTS =========================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_is_stable() {
        use core::mem::size_of;
        assert_eq!(size_of::<InstrumentType>(), 2);
        assert_eq!(size_of::<ModelKey>(), 2);
        assert_eq!(size_of::<PricerKey>(), 4);
    }

    #[test]
    fn registry_creation_test() {
        // Test that the standard registry can be created without errors
        let registry = create_standard_registry();

        // Test that we can retrieve a registered pricer
        let key = PricerKey::new(InstrumentType::Bond, ModelKey::Discounting);
        assert!(registry.get_pricer(key).is_some());
    }

    #[test]
    fn registration_covers_all_pricers() {
        // Test that ALL pricers are registered correctly
        let registry = create_standard_registry();

        // Bond pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Discounting))
                .is_some(),
            "Bond Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::Tree))
                .is_some(),
            "Bond OAS pricer should be registered"
        );

        // Interest Rate pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::IRS, ModelKey::Discounting))
                .is_some(),
            "IRS Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FRA, ModelKey::Discounting))
                .is_some(),
            "FRA Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76))
                .is_some(),
            "CapFloor Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CapFloor,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CapFloor Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Swaption, ModelKey::Black76))
                .is_some(),
            "Swaption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Swaption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Swaption Discounting pricer should be registered"
        );

        // Credit pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate))
                .is_some(),
            "CDS HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDS, ModelKey::Discounting))
                .is_some(),
            "CDS Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSIndex HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSIndex,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSIndex Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76))
                .is_some(),
            "CDSOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSOption Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::HazardRate
                ))
                .is_some(),
            "CDSTranche HazardRate pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CDSTranche,
                    ModelKey::Discounting
                ))
                .is_some(),
            "CDSTranche Discounting pricer should be registered"
        );

        // FX pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSpot,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSpot Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::FxOption, ModelKey::Black76))
                .is_some(),
            "FxOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxOption Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FxSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxSwap Discounting pricer should be registered"
        );

        // Equity pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Equity,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Equity Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Black76
                ))
                .is_some(),
            "EquityOption Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityOption,
                    ModelKey::Discounting
                ))
                .is_some(),
            "EquityOption Discounting pricer should be registered"
        );

        // Basic pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Deposit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Deposit Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InterestRateFuture,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InterestRateFuture Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::BasisSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "BasisSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::Repo, ModelKey::Discounting))
                .is_some(),
            "Repo Discounting pricer should be registered"
        );

        // Inflation pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationLinkedBond,
                    ModelKey::Discounting
                ))
                .is_some(),
            "InflationLinkedBond Discounting pricer should be registered"
        );

        // Complex pricers
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::VarianceSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "VarianceSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Basket,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Basket Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::Convertible,
                    ModelKey::Discounting
                ))
                .is_some(),
            "Convertible Discounting pricer should be registered"
        );

        // Structured credit pricer (unified for ABS, CLO, CMBS, RMBS)
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::StructuredCredit,
                    ModelKey::Discounting
                ))
                .is_some(),
            "StructuredCredit Discounting pricer should be registered"
        );

        // TRS and Private Markets
        assert!(
            registry
                .get_pricer(PricerKey::new(InstrumentType::TRS, ModelKey::Discounting))
                .is_some(),
            "TRS Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::PrivateMarketsFund,
                    ModelKey::Discounting
                ))
                .is_some(),
            "PrivateMarketsFund Discounting pricer should be registered"
        );
    }
}

//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.

use crate::instruments::common::traits::Instrument as Priceable;
use finstack_core::market_data::context::MarketContext as Market;

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
    /// Option on interest rate swap (European exercise).
    Swaption = 9,
    /// Bermudan swaption (multiple exercise dates).
    BermudanSwaption = 10,
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
    /// Cross-currency swap (multi-currency floating legs).
    XccySwap = 52,
    /// Inflation-linked bond (TIPS, index-linked gilts).
    InflationLinkedBond = 19,
    /// Zero-coupon inflation swap.
    InflationSwap = 20,
    /// Year-on-year inflation swap.
    YoYInflationSwap = 53,
    /// Inflation cap/floor (YoY inflation options).
    InflationCapFloor = 66,
    /// Interest rate futures contract.
    InterestRateFuture = 21,
    /// Variance swap (volatility exposure).
    VarianceSwap = 22,
    /// FX variance swap (volatility exposure on FX).
    FxVarianceSwap = 67,
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
    /// Discounted Cash Flow (corporate valuation)
    DCF = 42,
    /// Real estate asset valuation (DCF or direct cap).
    RealEstateAsset = 69,
    /// Equity Total Return Swap.
    EquityTotalReturnSwap = 50,
    /// Fixed Income Index Total Return Swap.
    FIIndexTotalReturnSwap = 51,
    /// Bond future (futures on a deliverable bond basket with CTD mechanics).
    BondFuture = 54,
    /// Commodity forward or futures contract.
    CommodityForward = 55,
    /// Commodity swap (fixed-for-floating commodity price exchange).
    CommoditySwap = 56,
    /// Commodity option (option on commodity forward or spot).
    CommodityOption = 68,
    /// Volatility index future (VIX, VXN, VSTOXX).
    VolatilityIndexFuture = 57,
    /// Volatility index option (options on VIX, etc.).
    VolatilityIndexOption = 58,
    /// Equity index future (ES, NQ, FESX, FDAX, Z, NK).
    EquityIndexFuture = 59,
    /// FX forward (outright forward, single exchange at maturity).
    FxForward = 60,
    /// Non-deliverable forward (NDF) for restricted currencies.
    Ndf = 61,
    /// Agency MBS passthrough (FNMA, FHLMC, GNMA pools).
    AgencyMbsPassthrough = 62,
    /// Agency TBA (To-Be-Announced) forward.
    AgencyTba = 63,
    /// Dollar roll (TBA financing trade).
    DollarRoll = 64,
    /// Agency CMO (Collateralized Mortgage Obligation).
    AgencyCmo = 65,
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
            InstrumentType::CapFloor => "CapFloor",
            InstrumentType::Swaption => "Swaption",
            InstrumentType::BermudanSwaption => "BermudanSwaption",
            InstrumentType::BasisSwap => "BasisSwap",
            InstrumentType::Basket => "Basket",
            InstrumentType::Convertible => "ConvertibleBond",
            InstrumentType::Deposit => "Deposit",
            InstrumentType::EquityOption => "EquityOption",
            InstrumentType::FxOption => "FxOption",
            InstrumentType::FxSpot => "FxSpot",
            InstrumentType::FxSwap => "FxSwap",
            InstrumentType::XccySwap => "XccySwap",
            InstrumentType::InflationLinkedBond => "InflationLinkedBond",
            InstrumentType::InflationSwap => "InflationSwap",
            InstrumentType::YoYInflationSwap => "YoYInflationSwap",
            InstrumentType::InflationCapFloor => "InflationCapFloor",
            InstrumentType::InterestRateFuture => "InterestRateFuture",
            InstrumentType::VarianceSwap => "VarianceSwap",
            InstrumentType::FxVarianceSwap => "FxVarianceSwap",
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
            InstrumentType::DCF => "DCF",
            InstrumentType::RealEstateAsset => "RealEstateAsset",
            InstrumentType::EquityTotalReturnSwap => "EquityTotalReturnSwap",
            InstrumentType::FIIndexTotalReturnSwap => "FIIndexTotalReturnSwap",
            InstrumentType::BondFuture => "BondFuture",
            InstrumentType::CommodityForward => "CommodityForward",
            InstrumentType::CommoditySwap => "CommoditySwap",
            InstrumentType::CommodityOption => "CommodityOption",
            InstrumentType::VolatilityIndexFuture => "VolatilityIndexFuture",
            InstrumentType::VolatilityIndexOption => "VolatilityIndexOption",
            InstrumentType::EquityIndexFuture => "EquityIndexFuture",
            InstrumentType::FxForward => "FxForward",
            InstrumentType::Ndf => "Ndf",
            InstrumentType::AgencyMbsPassthrough => "AgencyMbsPassthrough",
            InstrumentType::AgencyTba => "AgencyTba",
            InstrumentType::DollarRoll => "DollarRoll",
            InstrumentType::AgencyCmo => "AgencyCmo",
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
            InstrumentType::BermudanSwaption => "bermudan_swaption",
            InstrumentType::BasisSwap => "basis_swap",
            InstrumentType::Basket => "basket",
            InstrumentType::Convertible => "convertible",
            InstrumentType::Deposit => "deposit",
            InstrumentType::EquityOption => "equity_option",
            InstrumentType::FxOption => "fx_option",
            InstrumentType::FxSpot => "fx_spot",
            InstrumentType::FxSwap => "fx_swap",
            InstrumentType::XccySwap => "xccy_swap",
            InstrumentType::InflationLinkedBond => "inflation_linked_bond",
            InstrumentType::InflationSwap => "inflation_swap",
            InstrumentType::YoYInflationSwap => "yoy_inflation_swap",
            InstrumentType::InflationCapFloor => "inflation_cap_floor",
            InstrumentType::InterestRateFuture => "interest_rate_future",
            InstrumentType::VarianceSwap => "variance_swap",
            InstrumentType::FxVarianceSwap => "fx_variance_swap",
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
            InstrumentType::DCF => "dcf",
            InstrumentType::RealEstateAsset => "real_estate_asset",
            InstrumentType::EquityTotalReturnSwap => "equity_total_return_swap",
            InstrumentType::FIIndexTotalReturnSwap => "fi_index_total_return_swap",
            InstrumentType::BondFuture => "bond_future",
            InstrumentType::CommodityForward => "commodity_forward",
            InstrumentType::CommoditySwap => "commodity_swap",
            InstrumentType::CommodityOption => "commodity_option",
            InstrumentType::VolatilityIndexFuture => "volatility_index_future",
            InstrumentType::VolatilityIndexOption => "volatility_index_option",
            InstrumentType::EquityIndexFuture => "equity_index_future",
            InstrumentType::FxForward => "fx_forward",
            InstrumentType::Ndf => "ndf",
            InstrumentType::AgencyMbsPassthrough => "agency_mbs_passthrough",
            InstrumentType::AgencyTba => "agency_tba",
            InstrumentType::DollarRoll => "dollar_roll",
            InstrumentType::AgencyCmo => "agency_cmo",
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
            "bermudan_swaption" | "bermudanswaption" => Ok(InstrumentType::BermudanSwaption),
            "basis_swap" | "basisswap" => Ok(InstrumentType::BasisSwap),
            "basket" => Ok(InstrumentType::Basket),
            "convertible" | "convertible_bond" => Ok(InstrumentType::Convertible),
            "deposit" => Ok(InstrumentType::Deposit),
            "equity_option" | "equityoption" => Ok(InstrumentType::EquityOption),
            "fx_option" | "fxoption" => Ok(InstrumentType::FxOption),
            "fx_spot" | "fxspot" => Ok(InstrumentType::FxSpot),
            "fx_swap" | "fxswap" => Ok(InstrumentType::FxSwap),
            "xccy_swap" | "xccyswap" | "xccy" | "cross_currency_swap" => {
                Ok(InstrumentType::XccySwap)
            }
            "inflation_linked_bond" | "ilb" => Ok(InstrumentType::InflationLinkedBond),
            "inflation_swap" => Ok(InstrumentType::InflationSwap),
            "yoy_inflation_swap" | "yo_y_inflation_swap" | "inflation_yoy_swap" | "yoy_swap" => {
                Ok(InstrumentType::YoYInflationSwap)
            }
            "inflation_cap_floor" | "inflation_cap" | "inflation_floor" => {
                Ok(InstrumentType::InflationCapFloor)
            }
            "interest_rate_future" | "ir_future" | "irfuture" => {
                Ok(InstrumentType::InterestRateFuture)
            }
            "variance_swap" | "varianceswap" => Ok(InstrumentType::VarianceSwap),
            "fx_variance_swap" | "fxvarianceswap" => Ok(InstrumentType::FxVarianceSwap),
            "equity" => Ok(InstrumentType::Equity),
            "repo" => Ok(InstrumentType::Repo),
            "fra" => Ok(InstrumentType::FRA),
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
            "dcf" | "discounted_cash_flow" => Ok(InstrumentType::DCF),
            "real_estate_asset" | "real_estate" | "realestate" | "realestate_asset" => {
                Ok(InstrumentType::RealEstateAsset)
            }
            "equity_total_return_swap" | "equity_trs" | "equitytrs" => {
                Ok(InstrumentType::EquityTotalReturnSwap)
            }
            "fi_index_total_return_swap" | "fi_index_trs" | "fiindex_trs" | "fiindextrs" => {
                Ok(InstrumentType::FIIndexTotalReturnSwap)
            }
            "bond_future" | "bondfuture" => Ok(InstrumentType::BondFuture),
            "commodity_forward" | "commodityforward" | "commodity_future" | "commodityfuture" => {
                Ok(InstrumentType::CommodityForward)
            }
            "commodity_swap" | "commodityswap" => Ok(InstrumentType::CommoditySwap),
            "commodity_option" | "commodityoption" => Ok(InstrumentType::CommodityOption),
            "volatility_index_future" | "vol_index_future" | "vix_future" | "vixfuture" => {
                Ok(InstrumentType::VolatilityIndexFuture)
            }
            "volatility_index_option" | "vol_index_option" | "vix_option" | "vixoption" => {
                Ok(InstrumentType::VolatilityIndexOption)
            }
            "equity_index_future" | "equityindexfuture" | "eq_future" | "es_future" => {
                Ok(InstrumentType::EquityIndexFuture)
            }
            "fx_forward" | "fxforward" | "outright_forward" => Ok(InstrumentType::FxForward),
            "ndf" | "non_deliverable_forward" => Ok(InstrumentType::Ndf),
            "agency_mbs_passthrough" | "agency_mbs" | "mbs" | "passthrough" => {
                Ok(InstrumentType::AgencyMbsPassthrough)
            }
            "agency_tba" | "tba" => Ok(InstrumentType::AgencyTba),
            "dollar_roll" | "dollarroll" | "roll" => Ok(InstrumentType::DollarRoll),
            "agency_cmo" | "cmo" => Ok(InstrumentType::AgencyCmo),
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
    /// Normal (Bachelier) model variant.
    Normal = 6,
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
            ModelKey::Normal => "normal",
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
            "normal" | "bachelier" => Ok(ModelKey::Normal),
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

/// Context for pricing operations, providing actionable debugging information.
///
/// This struct captures the instrument, model, and market data context
/// when a pricing error occurs, enabling easier troubleshooting.
#[derive(Debug, Clone, Default)]
pub struct PricingErrorContext {
    /// The instrument ID that was being priced (if known).
    pub instrument_id: Option<String>,
    /// The instrument type being priced.
    pub instrument_type: Option<InstrumentType>,
    /// The pricing model being used.
    pub model: Option<ModelKey>,
    /// Market data curve/surface IDs involved in the operation.
    pub curve_ids: Vec<String>,
}

impl PricingErrorContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the instrument ID.
    pub fn with_instrument_id(mut self, id: impl Into<String>) -> Self {
        self.instrument_id = Some(id.into());
        self
    }

    /// Set the instrument type.
    pub fn with_instrument_type(mut self, typ: InstrumentType) -> Self {
        self.instrument_type = Some(typ);
        self
    }

    /// Set the pricing model.
    pub fn with_model(mut self, model: ModelKey) -> Self {
        self.model = Some(model);
        self
    }

    /// Add a curve/surface ID to the context.
    pub fn with_curve_id(mut self, curve_id: impl Into<String>) -> Self {
        self.curve_ids.push(curve_id.into());
        self
    }

    /// Add multiple curve/surface IDs to the context.
    pub fn with_curve_ids(
        mut self,
        curve_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.curve_ids
            .extend(curve_ids.into_iter().map(|s| s.into()));
        self
    }
}

impl std::fmt::Display for PricingErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(ref id) = self.instrument_id {
            parts.push(format!("instrument={}", id));
        }
        if let Some(typ) = self.instrument_type {
            parts.push(format!("type={:?}", typ));
        }
        if let Some(model) = self.model {
            parts.push(format!("model={:?}", model));
        }
        if !self.curve_ids.is_empty() {
            parts.push(format!("curves=[{}]", self.curve_ids.join(", ")));
        }
        if parts.is_empty() {
            write!(f, "<no context>")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

/// Pricing-specific errors returned by pricer implementations.
///
/// Each variant captures the error condition along with optional context
/// (instrument ID, type, model, and curve IDs) for actionable debugging.
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
    ///
    /// The context provides actionable information about which instrument
    /// and model were involved when the failure occurred.
    #[error("Model failure: {message}{}", format_context(.context))]
    ModelFailure {
        /// Error message describing the failure.
        message: String,
        /// Context: instrument, model, and curves involved.
        context: PricingErrorContext,
    },

    /// Invalid input parameters provided.
    ///
    /// The context identifies which instrument had invalid inputs.
    #[error("Invalid input: {message}{}", format_context(.context))]
    InvalidInput {
        /// Error message describing the invalid input.
        message: String,
        /// Context: instrument and relevant details.
        context: PricingErrorContext,
    },

    /// Missing market data required for pricing.
    ///
    /// Identifies exactly which market data ID is missing and for which instrument.
    #[error("Missing market data: {missing_id} required for pricing{}", format_context(.context))]
    MissingMarketData {
        /// The ID of the missing market data (curve, surface, or scalar).
        missing_id: String,
        /// Context: instrument requiring this data.
        context: PricingErrorContext,
    },
}

/// Helper to format context for error display.
fn format_context(ctx: &PricingErrorContext) -> String {
    let display = ctx.to_string();
    if display == "<no context>" {
        String::new()
    } else {
        format!(" [{}]", display)
    }
}

impl From<PricingError> for finstack_core::Error {
    fn from(err: PricingError) -> Self {
        finstack_core::Error::Validation(err.to_string())
    }
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        PricingError::ModelFailure {
            message: err.to_string(),
            context: PricingErrorContext::default(),
        }
    }
}

impl PricingError {
    /// Create a type mismatch error.
    pub fn type_mismatch(expected: InstrumentType, got: InstrumentType) -> Self {
        Self::TypeMismatch { expected, got }
    }

    /// Create a model failure error (backward compatible, no context).
    pub fn model_failure(msg: impl Into<String>) -> Self {
        Self::ModelFailure {
            message: msg.into(),
            context: PricingErrorContext::default(),
        }
    }

    /// Create a model failure error with full context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// PricingError::model_failure_ctx(
    ///     "Discount factor calculation failed",
    ///     PricingErrorContext::new()
    ///         .with_instrument_id("BOND-001")
    ///         .with_instrument_type(InstrumentType::Bond)
    ///         .with_model(ModelKey::Discounting)
    ///         .with_curve_id("USD-OIS"),
    /// )
    /// ```
    pub fn model_failure_ctx(msg: impl Into<String>, context: PricingErrorContext) -> Self {
        Self::ModelFailure {
            message: msg.into(),
            context,
        }
    }

    /// Create an invalid input error (backward compatible, no context).
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: msg.into(),
            context: PricingErrorContext::default(),
        }
    }

    /// Create an invalid input error with full context.
    pub fn invalid_input_ctx(msg: impl Into<String>, context: PricingErrorContext) -> Self {
        Self::InvalidInput {
            message: msg.into(),
            context,
        }
    }

    /// Create a missing market data error (backward compatible, takes description).
    pub fn missing_market_data(msg: impl Into<String>) -> Self {
        Self::MissingMarketData {
            missing_id: msg.into(),
            context: PricingErrorContext::default(),
        }
    }

    /// Create a missing market data error with the specific missing ID and context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// PricingError::missing_market_data_ctx(
    ///     "USD-OIS",
    ///     PricingErrorContext::new()
    ///         .with_instrument_id("BOND-001")
    ///         .with_instrument_type(InstrumentType::Bond),
    /// )
    /// ```
    pub fn missing_market_data_ctx(
        missing_id: impl Into<String>,
        context: PricingErrorContext,
    ) -> Self {
        Self::MissingMarketData {
            missing_id: missing_id.into(),
            context,
        }
    }

    /// Add context to an existing error.
    ///
    /// This is useful for enriching errors as they propagate up the call stack.
    pub fn with_context(self, context: PricingErrorContext) -> Self {
        match self {
            Self::ModelFailure { message, .. } => Self::ModelFailure { message, context },
            Self::InvalidInput { message, .. } => Self::InvalidInput { message, context },
            Self::MissingMarketData { missing_id, .. } => Self::MissingMarketData {
                missing_id,
                context,
            },
            // These variants already have context or don't need it
            other => other,
        }
    }

    /// Add instrument ID to the error context.
    pub fn with_instrument_id(self, id: impl Into<String>) -> Self {
        let id = id.into();
        match self {
            Self::ModelFailure {
                message,
                mut context,
            } => {
                context.instrument_id = Some(id);
                Self::ModelFailure { message, context }
            }
            Self::InvalidInput {
                message,
                mut context,
            } => {
                context.instrument_id = Some(id);
                Self::InvalidInput { message, context }
            }
            Self::MissingMarketData {
                missing_id,
                mut context,
            } => {
                context.instrument_id = Some(id);
                Self::MissingMarketData {
                    missing_id,
                    context,
                }
            }
            other => other,
        }
    }

    /// Add instrument type to the error context.
    pub fn with_instrument_type(self, typ: InstrumentType) -> Self {
        match self {
            Self::ModelFailure {
                message,
                mut context,
            } => {
                context.instrument_type = Some(typ);
                Self::ModelFailure { message, context }
            }
            Self::InvalidInput {
                message,
                mut context,
            } => {
                context.instrument_type = Some(typ);
                Self::InvalidInput { message, context }
            }
            Self::MissingMarketData {
                missing_id,
                mut context,
            } => {
                context.instrument_type = Some(typ);
                Self::MissingMarketData {
                    missing_id,
                    context,
                }
            }
            other => other,
        }
    }

    /// Add model key to the error context.
    pub fn with_model(self, model: ModelKey) -> Self {
        match self {
            Self::ModelFailure {
                message,
                mut context,
            } => {
                context.model = Some(model);
                Self::ModelFailure { message, context }
            }
            Self::InvalidInput {
                message,
                mut context,
            } => {
                context.model = Some(model);
                Self::InvalidInput { message, context }
            }
            Self::MissingMarketData {
                missing_id,
                mut context,
            } => {
                context.model = Some(model);
                Self::MissingMarketData {
                    missing_id,
                    context,
                }
            }
            other => other,
        }
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

use finstack_core::collections::HashMap;

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

    /// Helper to look up a pricer using distinct type and model.
    ///
    /// # Arguments
    ///
    /// * `inst` - Instrument type
    /// * `model` - Model key
    ///
    /// # Returns
    ///
    /// `Some(&dyn Pricer)` if registered, `None` otherwise
    pub fn get(&self, inst: InstrumentType, model: ModelKey) -> Option<&dyn Pricer> {
        self.get_pricer(PricerKey::new(inst, model))
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
    macro_rules! register_pricer {
        ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
            $registry.register_pricer(
                PricerKey::new(InstrumentType::$inst, ModelKey::$model),
                Box::new($pricer),
            );
        };
    }

    // Bond pricers
    register_pricer!(
        registry,
        Bond,
        Discounting,
        crate::instruments::bond::pricing::pricer::SimpleBondDiscountingPricer::default()
    );
    register_pricer!(
        registry,
        Bond,
        HazardRate,
        crate::instruments::bond::pricing::pricer::SimpleBondHazardPricer
    );
    register_pricer!(
        registry,
        Bond,
        Tree,
        crate::instruments::bond::pricing::pricer::SimpleBondOasPricer
    );

    // Interest Rate Swaps
    register_pricer!(
        registry,
        IRS,
        Discounting,
        crate::instruments::common::GenericDiscountingPricer::<
            crate::instruments::InterestRateSwap,
        >::new(InstrumentType::IRS)
    );

    // FRA
    register_pricer!(
        registry,
        FRA,
        Discounting,
        crate::instruments::fra::pricer::SimpleFraDiscountingPricer::default()
    );

    // Basis Swap
    register_pricer!(
        registry,
        BasisSwap,
        Discounting,
        crate::instruments::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default()
    );

    // Deposit
    register_pricer!(
        registry,
        Deposit,
        Discounting,
        crate::instruments::deposit::pricer::SimpleDepositDiscountingPricer::default()
    );

    // Interest Rate Future
    register_pricer!(
        registry,
        InterestRateFuture,
        Discounting,
        crate::instruments::ir_future::pricer::SimpleIrFutureDiscountingPricer::default()
    );

    // Bond Future
    register_pricer!(
        registry,
        BondFuture,
        Discounting,
        crate::instruments::bond_future::pricer::BondFuturePricer
    );

    // Cap/Floor
    register_pricer!(
        registry,
        CapFloor,
        Black76,
        crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default()
    );
    register_pricer!(
        registry,
        CapFloor,
        Discounting,
        crate::instruments::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Swaption
    register_pricer!(
        registry,
        Swaption,
        Black76,
        crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        Swaption,
        Discounting,
        crate::instruments::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS
    register_pricer!(
        registry,
        CDS,
        HazardRate,
        crate::instruments::common::GenericInstrumentPricer::cds()
    );
    register_pricer!(
        registry,
        CDS,
        Discounting,
        crate::instruments::common::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting)
    );

    // CDS Index
    register_pricer!(
        registry,
        CDSIndex,
        HazardRate,
        crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSIndex,
        Discounting,
        crate::instruments::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Tranche
    register_pricer!(
        registry,
        CDSTranche,
        HazardRate,
        crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSTranche,
        Discounting,
        crate::instruments::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Option
    register_pricer!(
        registry,
        CDSOption,
        Black76,
        crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CDSOption,
        Discounting,
        crate::instruments::cds_option::pricer::SimpleCdsOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // FX Spot
    register_pricer!(
        registry,
        FxSpot,
        Discounting,
        crate::instruments::fx_spot::pricer::FxSpotPricer
    );

    // FX Swap
    register_pricer!(
        registry,
        FxSwap,
        Discounting,
        crate::instruments::fx_swap::pricer::SimpleFxSwapDiscountingPricer::default()
    );

    // XCCY Swap
    register_pricer!(
        registry,
        XccySwap,
        Discounting,
        crate::instruments::xccy_swap::pricer::SimpleXccySwapDiscountingPricer::default()
    );

    // FX Option
    register_pricer!(
        registry,
        FxOption,
        Black76,
        crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        FxOption,
        Discounting,
        crate::instruments::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Equity
    register_pricer!(
        registry,
        Equity,
        Discounting,
        crate::instruments::equity::spot::pricer::SimpleEquityDiscountingPricer
    );

    // Equity Option
    register_pricer!(
        registry,
        EquityOption,
        Black76,
        crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        EquityOption,
        Discounting,
        crate::instruments::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        EquityOption,
        HestonFourier,
        crate::instruments::equity_option::pricer::EquityOptionHestonFourierPricer
    );

    // Equity TRS
    register_pricer!(
        registry,
        EquityTotalReturnSwap,
        Discounting,
        crate::instruments::common::GenericDiscountingPricer::<
            crate::instruments::equity_trs::EquityTotalReturnSwap,
        >::new(InstrumentType::EquityTotalReturnSwap)
    );

    // FI Index TRS
    register_pricer!(
        registry,
        FIIndexTotalReturnSwap,
        Discounting,
        crate::instruments::common::GenericDiscountingPricer::<
            crate::instruments::fi_trs::FIIndexTotalReturnSwap,
        >::new(InstrumentType::FIIndexTotalReturnSwap)
    );

    // Convertible Bond
    register_pricer!(
        registry,
        Convertible,
        Discounting,
        crate::instruments::convertible::pricer::SimpleConvertibleDiscountingPricer
    );

    // Private Markets Fund
    register_pricer!(
        registry,
        PrivateMarketsFund,
        Discounting,
        crate::instruments::private_markets_fund::pricer::PrivateMarketsFundDiscountingPricer
    );

    // Inflation Swap
    register_pricer!(
        registry,
        InflationSwap,
        Discounting,
        crate::instruments::inflation_swap::pricer::SimpleInflationSwapDiscountingPricer::default()
    );

    // YoY Inflation Swap
    register_pricer!(
        registry,
        YoYInflationSwap,
        Discounting,
        crate::instruments::common::GenericDiscountingPricer::<
            crate::instruments::inflation_swap::YoYInflationSwap,
        >::new(InstrumentType::YoYInflationSwap)
    );

    // Inflation Cap/Floor
    register_pricer!(
        registry,
        InflationCapFloor,
        Black76,
        crate::instruments::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::default()
    );
    register_pricer!(
        registry,
        InflationCapFloor,
        Normal,
        crate::instruments::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::with_model(
            ModelKey::Normal
        )
    );

    // Inflation Linked Bond
    register_pricer!(
        registry,
        InflationLinkedBond,
        Discounting,
        crate::instruments::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::default()
    );

    // Variance Swap
    register_pricer!(
        registry,
        VarianceSwap,
        Discounting,
        crate::instruments::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default()
    );

    // FX Variance Swap
    register_pricer!(
        registry,
        FxVarianceSwap,
        Discounting,
        crate::instruments::fx_variance_swap::pricer::SimpleFxVarianceSwapDiscountingPricer::default()
    );

    // Repo
    register_pricer!(
        registry,
        Repo,
        Discounting,
        crate::instruments::repo::pricer::SimpleRepoDiscountingPricer::default()
    );

    // Basket
    register_pricer!(
        registry,
        Basket,
        Discounting,
        crate::instruments::basket::SimpleBasketDiscountingPricer::default()
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    register_pricer!(
        registry,
        StructuredCredit,
        Discounting,
        crate::instruments::structured_credit::StructuredCreditDiscountingPricer::default()
    );

    // Revolving Credit
    register_pricer!(
        registry,
        RevolvingCredit,
        Discounting,
        crate::instruments::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::Discounting
        )
    );
    // Term Loan (including DDTL)
    register_pricer!(
        registry,
        TermLoan,
        Discounting,
        crate::instruments::term_loan::pricing::TermLoanDiscountingPricer
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RevolvingCredit,
        MonteCarloGBM,
        crate::instruments::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::MonteCarloGBM
        )
    );

    // Asian Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        AsianOption,
        MonteCarloGBM,
        crate::instruments::asian_option::pricer::AsianOptionMcPricer::default()
    );
    // Asian Option - Analytical (Geometric)
    register_pricer!(
        registry,
        AsianOption,
        AsianGeometricBS,
        crate::instruments::asian_option::pricer::AsianOptionAnalyticalGeometricPricer
    );
    // Asian Option - Semi-Analytical (Turnbull-Wakeman)
    register_pricer!(
        registry,
        AsianOption,
        AsianTurnbullWakeman,
        crate::instruments::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer
    );

    // Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BarrierOption,
        MonteCarloGBM,
        crate::instruments::barrier_option::pricer::BarrierOptionMcPricer::default()
    );
    // Barrier Option - Analytical (Continuous monitoring)
    register_pricer!(
        registry,
        BarrierOption,
        BarrierBSContinuous,
        crate::instruments::barrier_option::pricer::BarrierOptionAnalyticalPricer
    );

    // Lookback Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        LookbackOption,
        MonteCarloGBM,
        crate::instruments::lookback_option::pricer::LookbackOptionMcPricer::default()
    );
    // Lookback Option - Analytical (Continuous monitoring)
    register_pricer!(
        registry,
        LookbackOption,
        LookbackBSContinuous,
        crate::instruments::lookback_option::pricer::LookbackOptionAnalyticalPricer
    );

    // Quanto Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        QuantoOption,
        MonteCarloGBM,
        crate::instruments::quanto_option::pricer::QuantoOptionMcPricer::default()
    );
    // Quanto Option - Analytical
    register_pricer!(
        registry,
        QuantoOption,
        QuantoBS,
        crate::instruments::quanto_option::pricer::QuantoOptionAnalyticalPricer
    );

    // Autocallable
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        Autocallable,
        MonteCarloGBM,
        crate::instruments::autocallable::pricer::AutocallableMcPricer::default()
    );

    // CMS Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CmsOption,
        MonteCarloHullWhite1F,
        crate::instruments::cms_option::pricer::CmsOptionPricer::new()
    );

    // Cliquet Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CliquetOption,
        MonteCarloGBM,
        crate::instruments::cliquet_option::pricer::CliquetOptionMcPricer::default()
    );

    // Range Accrual
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RangeAccrual,
        MonteCarloGBM,
        crate::instruments::range_accrual::pricer::RangeAccrualMcPricer::default()
    );

    // FX Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        FxBarrierOption,
        MonteCarloGBM,
        crate::instruments::fx_barrier_option::pricer::FxBarrierOptionMcPricer::default()
    );
    // FX Barrier Option - Analytical (Continuous monitoring)
    register_pricer!(
        registry,
        FxBarrierOption,
        FxBarrierBSContinuous,
        crate::instruments::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer
    );

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo)
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BermudanSwaption,
        MonteCarloHullWhite1F,
        crate::instruments::swaption::pricer::BermudanSwaptionPricer::lsmc_pricer(
            crate::instruments::swaption::pricer::HullWhiteParams::default()
        )
    );

    // DCF (Discounted Cash Flow)
    register_pricer!(
        registry,
        DCF,
        Discounting,
        crate::instruments::dcf::pricer::DcfPricer
    );

    // Real Estate Asset
    register_pricer!(
        registry,
        RealEstateAsset,
        Discounting,
        crate::instruments::equity::real_estate::pricer::RealEstateAssetDiscountingPricer
    );

    // Commodity Forward
    register_pricer!(
        registry,
        CommodityForward,
        Discounting,
        crate::instruments::commodity_forward::CommodityForwardDiscountingPricer
    );

    // Commodity Swap
    register_pricer!(
        registry,
        CommoditySwap,
        Discounting,
        crate::instruments::commodity_swap::CommoditySwapDiscountingPricer
    );

    // Commodity Option
    register_pricer!(
        registry,
        CommodityOption,
        Black76,
        crate::instruments::commodity_option::pricer::CommodityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CommodityOption,
        Discounting,
        crate::instruments::commodity_option::pricer::CommodityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Equity Index Future
    register_pricer!(
        registry,
        EquityIndexFuture,
        Discounting,
        crate::instruments::equity_index_future::EquityIndexFutureDiscountingPricer
    );

    // FX Forward
    register_pricer!(
        registry,
        FxForward,
        Discounting,
        crate::instruments::fx_forward::FxForwardDiscountingPricer
    );

    // NDF (Non-Deliverable Forward)
    register_pricer!(
        registry,
        Ndf,
        Discounting,
        crate::instruments::ndf::NdfDiscountingPricer
    );

    // Agency MBS Passthrough
    register_pricer!(
        registry,
        AgencyMbsPassthrough,
        Discounting,
        crate::instruments::agency_mbs_passthrough::AgencyMbsDiscountingPricer
    );

    // Agency TBA
    register_pricer!(
        registry,
        AgencyTba,
        Discounting,
        crate::instruments::agency_tba::AgencyTbaDiscountingPricer
    );

    // Dollar Roll
    register_pricer!(
        registry,
        DollarRoll,
        Discounting,
        crate::instruments::dollar_roll::DollarRollDiscountingPricer
    );

    // Agency CMO
    register_pricer!(
        registry,
        AgencyCmo,
        Discounting,
        crate::instruments::agency_cmo::AgencyCmoDiscountingPricer
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
#[allow(clippy::expect_used, clippy::panic)]
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

        // Test convenience method
        assert!(registry
            .get(InstrumentType::Bond, ModelKey::Discounting)
            .is_some());
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
                .get_pricer(PricerKey::new(InstrumentType::Bond, ModelKey::HazardRate))
                .is_some(),
            "Bond HazardRate pricer should be registered"
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
                    InstrumentType::YoYInflationSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "YoYInflationSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationCapFloor,
                    ModelKey::Black76
                ))
                .is_some(),
            "InflationCapFloor Black76 pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::InflationCapFloor,
                    ModelKey::Normal
                ))
                .is_some(),
            "InflationCapFloor Normal pricer should be registered"
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
                    InstrumentType::FxVarianceSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FxVarianceSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::RealEstateAsset,
                    ModelKey::Discounting
                ))
                .is_some(),
            "RealEstateAsset Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::CommodityOption,
                    ModelKey::Black76
                ))
                .is_some(),
            "CommodityOption Black76 pricer should be registered"
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
                .get_pricer(PricerKey::new(
                    InstrumentType::EquityTotalReturnSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "EquityTotalReturnSwap Discounting pricer should be registered"
        );
        assert!(
            registry
                .get_pricer(PricerKey::new(
                    InstrumentType::FIIndexTotalReturnSwap,
                    ModelKey::Discounting
                ))
                .is_some(),
            "FIIndexTotalReturnSwap Discounting pricer should be registered"
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

    #[test]
    fn test_instrument_type_bond_future() {
        let inst_type = InstrumentType::BondFuture;
        assert_eq!(inst_type.as_str(), "BondFuture");
        assert_eq!(format!("{}", inst_type), "bond_future");
        assert_eq!(inst_type as u16, 54);
    }
}

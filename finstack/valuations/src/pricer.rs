//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.

use crate::instruments::common_impl::traits::Instrument as Priceable;
use finstack_core::config::{results_meta_now, FinstackConfig};
use finstack_core::market_data::context::MarketContext as Market;

// ========================= KEYS =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// Commodity Asian option (option on arithmetic/geometric average of commodity prices).
    CommodityAsianOption = 72,
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
    /// FX digital (binary) option (cash-or-nothing / asset-or-nothing).
    FxDigitalOption = 70,
    /// FX touch option (one-touch / no-touch American binary).
    FxTouchOption = 71,
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
            InstrumentType::CommodityAsianOption => "CommodityAsianOption",
            InstrumentType::VolatilityIndexFuture => "VolatilityIndexFuture",
            InstrumentType::VolatilityIndexOption => "VolatilityIndexOption",
            InstrumentType::EquityIndexFuture => "EquityIndexFuture",
            InstrumentType::FxForward => "FxForward",
            InstrumentType::Ndf => "Ndf",
            InstrumentType::AgencyMbsPassthrough => "AgencyMbsPassthrough",
            InstrumentType::AgencyTba => "AgencyTba",
            InstrumentType::DollarRoll => "DollarRoll",
            InstrumentType::AgencyCmo => "AgencyCmo",
            InstrumentType::FxDigitalOption => "FxDigitalOption",
            InstrumentType::FxTouchOption => "FxTouchOption",
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
            InstrumentType::CommodityAsianOption => "commodity_asian_option",
            InstrumentType::VolatilityIndexFuture => "volatility_index_future",
            InstrumentType::VolatilityIndexOption => "volatility_index_option",
            InstrumentType::EquityIndexFuture => "equity_index_future",
            InstrumentType::FxForward => "fx_forward",
            InstrumentType::Ndf => "ndf",
            InstrumentType::AgencyMbsPassthrough => "agency_mbs_passthrough",
            InstrumentType::AgencyTba => "agency_tba",
            InstrumentType::DollarRoll => "dollar_roll",
            InstrumentType::AgencyCmo => "agency_cmo",
            InstrumentType::FxDigitalOption => "fx_digital_option",
            InstrumentType::FxTouchOption => "fx_touch_option",
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
            "fx_digital_option" | "fxdigitaloption" | "fx_digital" | "digital_option" => {
                Ok(InstrumentType::FxDigitalOption)
            }
            "fx_touch_option" | "fxtouchoption" | "fx_touch" | "touch_option" | "one_touch"
            | "no_touch" => Ok(InstrumentType::FxTouchOption),
            other => Err(format!("Unknown instrument type: {}", other)),
        }
    }
}

/// Pricing model selection for the pricer registry.
///
/// Determines which mathematical model is used to price an instrument.
/// Each model has different computational characteristics and accuracy
/// profiles for different instrument types.
///
/// # Model Categories
///
/// ## Analytical Models
/// - [`Discounting`](Self::Discounting): Simple present value discounting
/// - [`Black76`](Self::Black76): Black-76 formula for options
/// - [`Normal`](Self::Normal): Bachelier (normal) model for rate options
///
/// ## Tree Models
/// - [`Tree`](Self::Tree): Binomial/trinomial lattice
/// - [`HullWhite1F`](Self::HullWhite1F): Hull-White one-factor short rate
///
/// ## Monte Carlo Models
/// - [`MonteCarloGBM`](Self::MonteCarloGBM): GBM simulation
/// - [`MonteCarloHeston`](Self::MonteCarloHeston): Heston stochastic vol
///
/// ## Exotic Analytical
/// - [`BarrierBSContinuous`](Self::BarrierBSContinuous): Reiner-Rubinstein barriers
/// - [`AsianGeometricBS`](Self::AsianGeometricBS): Geometric Asian (exact)
/// - [`AsianTurnbullWakeman`](Self::AsianTurnbullWakeman): Arithmetic Asian (approx)
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::pricer::ModelKey;
///
/// // Select appropriate model for instrument type
/// let model = ModelKey::Discounting;  // For bonds
/// let model = ModelKey::Black76;      // For caps/floors
/// let model = ModelKey::MonteCarloGBM; // For path-dependent exotics
///
/// // Parse from string
/// let model: ModelKey = "black76".parse().unwrap();
/// assert_eq!(model, ModelKey::Black76);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum ModelKey {
    /// Present value discounting of projected cashflows.
    ///
    /// Used for: bonds, swaps, deposits, repos, forwards.
    Discounting = 1,
    /// Binomial or trinomial tree for callable/putable instruments.
    ///
    /// Used for: callable bonds, Bermudan options.
    Tree = 2,
    /// Black-76 lognormal model for forward-based options.
    ///
    /// Used for: caps, floors, swaptions, FX options, commodity options.
    Black76 = 3,
    /// Hull-White one-factor short rate model.
    ///
    /// Used for: callable bonds with OAS, Bermudan swaptions.
    HullWhite1F = 4,
    /// Hazard rate model for credit instruments.
    ///
    /// Used for: CDS, CDS indices, credit risky bonds.
    HazardRate = 5,
    /// Bachelier (normal) model for rate options.
    ///
    /// Used for: inflation caps/floors, options on rates near zero.
    Normal = 6,
    /// Monte Carlo with Geometric Brownian Motion.
    ///
    /// Used for: path-dependent options, exotics requiring simulation.
    MonteCarloGBM = 10,
    /// Monte Carlo with Heston stochastic volatility.
    ///
    /// Used for: options requiring volatility smile dynamics.
    MonteCarloHeston = 11,
    /// Monte Carlo with Hull-White 1F rates.
    ///
    /// Used for: Bermudan swaptions, CMS options.
    MonteCarloHullWhite1F = 12,
    /// Reiner-Rubinstein continuous barrier formulas.
    ///
    /// Used for: equity/FX barrier options with continuous monitoring.
    BarrierBSContinuous = 20,
    /// Kemna-Vorst exact geometric Asian formula.
    ///
    /// Used for: geometric average Asian options.
    AsianGeometricBS = 21,
    /// Turnbull-Wakeman approximation for arithmetic Asians.
    ///
    /// Used for: arithmetic average Asian options.
    AsianTurnbullWakeman = 22,
    /// Conze-Viswanathan lookback option formulas.
    ///
    /// Used for: lookback options with continuous monitoring.
    LookbackBSContinuous = 23,
    /// Quanto BS with drift adjustment.
    ///
    /// Used for: quanto options (cross-currency).
    QuantoBS = 24,
    /// FX barrier with Reiner-Rubinstein mapping.
    ///
    /// Used for: FX barrier options.
    FxBarrierBSContinuous = 25,
    /// Heston semi-analytical via Fourier transform.
    ///
    /// Used for: European options requiring stochastic vol.
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

/// Composite key identifying a specific (instrument, model) pricing combination.
///
/// The pricer registry uses `PricerKey` to dispatch instruments to the
/// appropriate pricer implementation. Each unique combination of instrument
/// type and pricing model maps to exactly one pricer.
///
/// # Layout
///
/// Uses `#[repr(C)]` for stable memory layout (4 bytes total: 2 for each enum).
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::pricer::{PricerKey, InstrumentType, ModelKey};
///
/// // Create key for bond discounting
/// let key = PricerKey::new(InstrumentType::Bond, ModelKey::Discounting);
///
/// // Create key for equity option Black-76
/// let key = PricerKey::new(InstrumentType::EquityOption, ModelKey::Black76);
///
/// // Keys are hashable for registry lookup
/// assert_eq!(key.instrument, InstrumentType::EquityOption);
/// assert_eq!(key.model, ModelKey::Black76);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PricerKey {
    /// The instrument type being priced.
    pub instrument: InstrumentType,
    /// The pricing model to use.
    pub model: ModelKey,
}

impl PricerKey {
    /// Create a new pricer key from instrument type and model.
    ///
    /// This is a const function, so it can be used in static contexts.
    ///
    /// # Arguments
    ///
    /// * `instrument` - The type of instrument to price
    /// * `model` - The pricing model to use
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::pricer::{PricerKey, InstrumentType, ModelKey};
    ///
    /// const BOND_DISCOUNT_KEY: PricerKey = PricerKey::new(
    ///     InstrumentType::Bond,
    ///     ModelKey::Discounting,
    /// );
    /// ```
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
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
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

    /// Create context from an instrument, capturing ID and type.
    ///
    /// This is a convenience method to reduce boilerplate when building
    /// error context in pricer implementations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ctx = PricingErrorContext::from_instrument(bond)
    ///     .model(ModelKey::Discounting)
    ///     .curve_id("USD-OIS");
    /// ```
    pub fn from_instrument(instrument: &dyn Priceable) -> Self {
        Self {
            instrument_id: Some(instrument.id().to_string()),
            instrument_type: Some(instrument.key()),
            ..Default::default()
        }
    }

    /// Set the instrument ID.
    pub fn instrument_id(mut self, id: impl Into<String>) -> Self {
        self.instrument_id = Some(id.into());
        self
    }

    /// Set the instrument type.
    pub fn instrument_type(mut self, typ: InstrumentType) -> Self {
        self.instrument_type = Some(typ);
        self
    }

    /// Set the pricing model.
    pub fn model(mut self, model: ModelKey) -> Self {
        self.model = Some(model);
        self
    }

    /// Add a curve/surface ID to the context.
    pub fn curve_id(mut self, curve_id: impl Into<String>) -> Self {
        self.curve_ids.push(curve_id.into());
        self
    }

    /// Add multiple curve/surface IDs to the context.
    pub fn curve_ids(mut self, curve_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.curve_ids
            .extend(curve_ids.into_iter().map(|s| s.into()));
        self
    }

    // -- Deprecated aliases for naming consistency --

    /// Deprecated: use [`instrument_id`](Self::instrument_id) instead.
    #[deprecated(
        since = "0.8.0",
        note = "renamed to `instrument_id` for naming consistency"
    )]
    pub fn with_instrument_id(self, id: impl Into<String>) -> Self {
        self.instrument_id(id)
    }

    /// Deprecated: use [`instrument_type`](Self::instrument_type) instead.
    #[deprecated(
        since = "0.8.0",
        note = "renamed to `instrument_type` for naming consistency"
    )]
    pub fn with_instrument_type(self, typ: InstrumentType) -> Self {
        self.instrument_type(typ)
    }

    /// Deprecated: use [`model`](Self::model) instead.
    #[deprecated(since = "0.8.0", note = "renamed to `model` for naming consistency")]
    pub fn with_model(self, model: ModelKey) -> Self {
        self.model(model)
    }

    /// Deprecated: use [`curve_id`](Self::curve_id) instead.
    #[deprecated(since = "0.8.0", note = "renamed to `curve_id` for naming consistency")]
    pub fn with_curve_id(self, curve_id: impl Into<String>) -> Self {
        self.curve_id(curve_id)
    }

    /// Deprecated: use [`curve_ids`](Self::curve_ids) instead.
    #[deprecated(
        since = "0.8.0",
        note = "renamed to `curve_ids` for naming consistency"
    )]
    pub fn with_curve_ids(self, curve_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.curve_ids(curve_ids)
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
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
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
        match err {
            PricingError::UnknownPricer(key) => {
                let pricer_id = format!("pricer:{}:{:?}", key.instrument.as_str(), key.model);
                finstack_core::InputError::NotFound { id: pricer_id }.into()
            }
            PricingError::TypeMismatch { .. } => finstack_core::InputError::Invalid.into(),
            // InvalidInput maps to Validation rather than Calibration:
            // these are input validation failures, not numerical/solver failures.
            PricingError::InvalidInput { message, context } => {
                finstack_core::Error::Validation(format!("{message}{}", format_context(&context)))
            }
            PricingError::MissingMarketData { missing_id, .. } => {
                finstack_core::InputError::NotFound {
                    id: missing_id.clone(),
                }
                .into()
            }
            // ModelFailure maps to Calibration for numerical/convergence failures.
            // This includes solver non-convergence, matrix singularity, etc.
            PricingError::ModelFailure { message, context } => finstack_core::Error::Calibration {
                message: format!("{message}{}", format_context(&context)),
                category: "pricing_model".to_string(),
            },
        }
    }
}

impl From<finstack_core::Error> for PricingError {
    fn from(err: finstack_core::Error) -> Self {
        match err {
            finstack_core::Error::Input(input) => match input {
                finstack_core::InputError::NotFound { id } => PricingError::MissingMarketData {
                    missing_id: id,
                    context: PricingErrorContext::default(),
                },
                finstack_core::InputError::MissingCurve { requested, .. } => {
                    PricingError::MissingMarketData {
                        missing_id: requested,
                        context: PricingErrorContext::default(),
                    }
                }
                finstack_core::InputError::WrongCurveType {
                    id,
                    expected,
                    actual,
                } => PricingError::InvalidInput {
                    message: format!(
                        "Curve type mismatch for '{id}': expected '{expected}', got '{actual}'"
                    ),
                    context: PricingErrorContext::default(),
                },
                other => PricingError::InvalidInput {
                    message: other.to_string(),
                    context: PricingErrorContext::default(),
                },
            },
            finstack_core::Error::Validation(msg) => PricingError::InvalidInput {
                message: msg,
                context: PricingErrorContext::default(),
            },
            finstack_core::Error::Calibration { message, .. } => PricingError::ModelFailure {
                message,
                context: PricingErrorContext::default(),
            },
            other => PricingError::ModelFailure {
                message: other.to_string(),
                context: PricingErrorContext::default(),
            },
        }
    }
}

impl PricingError {
    /// Create a type mismatch error.
    pub fn type_mismatch(expected: InstrumentType, got: InstrumentType) -> Self {
        Self::TypeMismatch { expected, got }
    }

    /// Create a model failure error with full context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// PricingError::model_failure_ctx(
    ///     "Discount factor calculation failed",
    ///     PricingErrorContext::new()
    ///         .instrument_id("BOND-001")
    ///         .instrument_type(InstrumentType::Bond)
    ///         .model(ModelKey::Discounting)
    ///         .curve_id("USD-OIS"),
    /// )
    /// ```
    pub fn model_failure_ctx(msg: impl Into<String>, context: PricingErrorContext) -> Self {
        Self::ModelFailure {
            message: msg.into(),
            context,
        }
    }

    /// Create an invalid input error with full context.
    pub fn invalid_input_ctx(msg: impl Into<String>, context: PricingErrorContext) -> Self {
        Self::InvalidInput {
            message: msg.into(),
            context,
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
    ///         .instrument_id("BOND-001")
    ///         .instrument_type(InstrumentType::Bond),
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

/// Extension trait for adding pricing context to Result types.
///
/// This trait provides a fluent API for attaching pricing context to errors,
/// similar to `anyhow::Context` but specialized for pricing operations.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::pricer::{PricingContextExt, InstrumentType};
///
/// let df = disc.df(maturity)
///     .with_pricing_context("BOND-001", InstrumentType::Bond, "discount factor")?;
/// ```
pub trait PricingContextExt<T> {
    /// Attach pricing context to an error.
    fn with_pricing_context(
        self,
        instrument_id: &str,
        instrument_type: InstrumentType,
        operation: &str,
    ) -> PricingResult<T>;
}

impl<T> PricingContextExt<T> for finstack_core::Result<T> {
    fn with_pricing_context(
        self,
        instrument_id: &str,
        instrument_type: InstrumentType,
        operation: &str,
    ) -> PricingResult<T> {
        self.map_err(|e| {
            PricingError::model_failure_ctx(
                format!("{}: {}", operation, e),
                PricingErrorContext::new()
                    .instrument_id(instrument_id)
                    .instrument_type(instrument_type),
            )
        })
    }
}

impl<T> PricingContextExt<T> for PricingResult<T> {
    fn with_pricing_context(
        self,
        instrument_id: &str,
        instrument_type: InstrumentType,
        operation: &str,
    ) -> PricingResult<T> {
        self.map_err(|e| {
            let context = PricingErrorContext::new()
                .instrument_id(instrument_id)
                .instrument_type(instrument_type);
            match e {
                PricingError::ModelFailure { message, .. } => {
                    PricingError::model_failure_ctx(format!("{}: {}", operation, message), context)
                }
                other => other.with_context(context),
            }
        })
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

use finstack_core::HashMap;

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
    /// * `cfg` - Optional FinstackConfig. When `Some`, the result will be stamped with
    ///   the exact rounding/tolerance policy from the config. When `None`, uses default config.
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
        cfg: Option<&FinstackConfig>,
    ) -> PricingResult<crate::results::ValuationResult> {
        let key = PricerKey::new(instrument.key(), model);
        let Some(pricer) = self.get_pricer(key) else {
            return Err(PricingError::UnknownPricer(key));
        };

        let mut result = pricer.price_dyn(instrument, market, as_of)?;
        let effective_cfg = cfg.map_or_else(FinstackConfig::default, |c| c.clone());
        stamp_results_meta(&effective_cfg, &mut result);
        Ok(result)
    }

    /// Price a batch of instruments using the registry dispatch system.
    ///
    /// The output order matches the input order. When the `parallel` feature is
    /// enabled, pricing is performed in parallel while preserving ordering.
    ///
    /// # Arguments
    ///
    /// * `instruments` - Slice of instruments to price (as trait objects)
    /// * `model` - Pricing model to use
    /// * `market` - Market data context with curves and surfaces
    /// * `as_of` - Valuation date
    /// * `cfg` - Optional FinstackConfig. When `Some`, results will be stamped with
    ///   the exact rounding/tolerance policy from the config. When `None`, uses default config.
    pub fn price_batch(
        &self,
        instruments: &[&dyn Priceable],
        model: ModelKey,
        market: &Market,
        as_of: finstack_core::dates::Date,
        cfg: Option<&FinstackConfig>,
    ) -> Vec<PricingResult<crate::results::ValuationResult>> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            instruments
                .par_iter()
                .map(|&instrument| self.price_with_registry(instrument, model, market, as_of, cfg))
                .collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            instruments
                .iter()
                .map(|&instrument| self.price_with_registry(instrument, model, market, as_of, cfg))
                .collect()
        }
    }
}

/// Stamp result metadata from a config, preserving FX policy stamps if present.
fn stamp_results_meta(cfg: &FinstackConfig, result: &mut crate::results::ValuationResult) {
    let prev_fx_policy = result.meta.fx_policy_applied.clone();
    let mut meta = results_meta_now(cfg);
    meta.fx_policy_applied = prev_fx_policy;
    result.meta = meta;
}

// ========================= REGISTRATION =========================

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register all standard pricers explicitly.
///
/// This function registers all instrument pricers in a single, visible location.
/// This explicit approach provides better IDE support, easier debugging, and
/// clearer dependency tracking compared to auto-registration. Registration here
/// is explicit and centralized, not implicit or macro-driven.
fn register_all_pricers(registry: &mut PricerRegistry) {
    // Group registries are the single source of truth for:
    // - rates minimal set (bonds + common rates)
    // - credit, equity, FX
    register_rates_pricers(registry);
    register_credit_pricers(registry);
    register_equity_pricers(registry);
    register_fx_pricers(registry);

    // Additional instrument pricers that are intentionally *not* included in the minimal group
    // registries above (e.g. to keep WASM footprints low).

    // FI Index TRS
    register_pricer!(
        registry,
        FIIndexTotalReturnSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap,
        >::discounting(InstrumentType::FIIndexTotalReturnSwap)
    );

    // Convertible Bond
    register_pricer!(
        registry,
        Convertible,
        Discounting,
        crate::instruments::fixed_income::convertible::pricer::SimpleConvertibleDiscountingPricer
    );

    // Private Markets Fund
    register_pricer!(
        registry,
        PrivateMarketsFund,
        Discounting,
        crate::instruments::equity::pe_fund::pricer::PrivateMarketsFundDiscountingPricer
    );

    // Inflation Swap
    register_pricer!(
        registry,
        InflationSwap,
        Discounting,
        crate::instruments::rates::inflation_swap::pricer::SimpleInflationSwapDiscountingPricer::default()
    );

    // YoY Inflation Swap
    register_pricer!(
        registry,
        YoYInflationSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::rates::inflation_swap::YoYInflationSwap,
        >::discounting(InstrumentType::YoYInflationSwap)
    );

    // Inflation Cap/Floor
    register_pricer!(
        registry,
        InflationCapFloor,
        Black76,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::default()
    );
    register_pricer!(
        registry,
        InflationCapFloor,
        Normal,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::with_model(
            ModelKey::Normal
        )
    );

    // Inflation Linked Bond
    register_pricer!(
        registry,
        InflationLinkedBond,
        Discounting,
        crate::instruments::fixed_income::inflation_linked_bond::pricer::SimpleInflationLinkedBondDiscountingPricer::default()
    );

    // Basket
    register_pricer!(
        registry,
        Basket,
        Discounting,
        crate::instruments::exotics::basket::SimpleBasketDiscountingPricer::default()
    );

    // Revolving Credit
    register_pricer!(
        registry,
        RevolvingCredit,
        Discounting,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::Discounting
        )
    );
    // Term Loan (including DDTL)
    register_pricer!(
        registry,
        TermLoan,
        Discounting,
        crate::instruments::fixed_income::term_loan::pricing::TermLoanDiscountingPricer
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RevolvingCredit,
        MonteCarloGBM,
        crate::instruments::fixed_income::revolving_credit::pricer::RevolvingCreditPricer::new(
            ModelKey::MonteCarloGBM
        )
    );

    // Asian Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        AsianOption,
        MonteCarloGBM,
        crate::instruments::exotics::asian_option::pricer::AsianOptionMcPricer::default()
    );
    // Asian Option - Analytical (Geometric)
    register_pricer!(
        registry,
        AsianOption,
        AsianGeometricBS,
        crate::instruments::exotics::asian_option::pricer::AsianOptionAnalyticalGeometricPricer
    );
    // Asian Option - Semi-Analytical (Turnbull-Wakeman)
    register_pricer!(
        registry,
        AsianOption,
        AsianTurnbullWakeman,
        crate::instruments::exotics::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer
    );

    // Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BarrierOption,
        MonteCarloGBM,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionMcPricer::default()
    );
    // Barrier Option - Analytical (Continuous monitoring)
    register_pricer!(
        registry,
        BarrierOption,
        BarrierBSContinuous,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionAnalyticalPricer
    );

    // Lookback Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        LookbackOption,
        MonteCarloGBM,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionMcPricer::default()
    );
    // Lookback Option - Analytical (Continuous monitoring)
    register_pricer!(
        registry,
        LookbackOption,
        LookbackBSContinuous,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionAnalyticalPricer
    );

    // Quanto Option - Analytical
    register_pricer!(
        registry,
        QuantoOption,
        QuantoBS,
        crate::instruments::fx::quanto_option::pricer::QuantoOptionAnalyticalPricer
    );

    // Autocallable
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        Autocallable,
        MonteCarloGBM,
        crate::instruments::equity::autocallable::pricer::AutocallableMcPricer::default()
    );

    // CMS Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CmsOption,
        MonteCarloHullWhite1F,
        crate::instruments::rates::cms_option::pricer::CmsOptionPricer::new()
    );

    // Cliquet Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CliquetOption,
        MonteCarloGBM,
        crate::instruments::equity::cliquet_option::pricer::CliquetOptionMcPricer::default()
    );

    // Range Accrual
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RangeAccrual,
        MonteCarloGBM,
        crate::instruments::rates::range_accrual::pricer::RangeAccrualMcPricer::default()
    );

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo)
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BermudanSwaption,
        MonteCarloHullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::lsmc_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default()
        )
    );

    // Commodity Forward - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        CommodityForward,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommodityForward,
        >::discounting(InstrumentType::CommodityForward)
    );

    // Commodity Swap - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        CommoditySwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommoditySwap,
        >::discounting(InstrumentType::CommoditySwap)
    );

    // Commodity Option
    register_pricer!(
        registry,
        CommodityOption,
        Black76,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CommodityOption,
        Discounting,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Agency MBS Passthrough
    register_pricer!(
        registry,
        AgencyMbsPassthrough,
        Discounting,
        crate::instruments::fixed_income::mbs_passthrough::AgencyMbsDiscountingPricer
    );

    // Agency TBA
    register_pricer!(
        registry,
        AgencyTba,
        Discounting,
        crate::instruments::fixed_income::tba::AgencyTbaDiscountingPricer
    );

    // Dollar Roll
    register_pricer!(
        registry,
        DollarRoll,
        Discounting,
        crate::instruments::fixed_income::dollar_roll::DollarRollDiscountingPricer
    );

    // Agency CMO
    register_pricer!(
        registry,
        AgencyCmo,
        Discounting,
        crate::instruments::fixed_income::cmo::AgencyCmoDiscountingPricer
    );
}

/// Register a minimal set of pricers for rates instruments.
///
/// Intended for environments (like WASM) where registering *all* pricers may be
/// too memory intensive.
pub fn register_rates_pricers(registry: &mut PricerRegistry) {
    // Bond pricers
    register_pricer!(
        registry,
        Bond,
        Discounting,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondDiscountingPricer::default()
    );
    register_pricer!(
        registry,
        Bond,
        HazardRate,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondHazardPricer
    );
    register_pricer!(
        registry,
        Bond,
        Tree,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondOasPricer
    );

    // Interest Rate Swaps
    register_pricer!(
        registry,
        IRS,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::InterestRateSwap,
        >::discounting(InstrumentType::IRS)
    );

    // FRA
    register_pricer!(
        registry,
        FRA,
        Discounting,
        crate::instruments::rates::fra::pricer::SimpleFraDiscountingPricer::default()
    );

    // Basis Swap
    register_pricer!(
        registry,
        BasisSwap,
        Discounting,
        crate::instruments::rates::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default()
    );

    // Deposit
    register_pricer!(
        registry,
        Deposit,
        Discounting,
        crate::instruments::rates::deposit::pricer::SimpleDepositDiscountingPricer::default()
    );

    // Interest Rate Future
    register_pricer!(
        registry,
        InterestRateFuture,
        Discounting,
        crate::instruments::rates::ir_future::pricer::SimpleIrFutureDiscountingPricer::default()
    );

    // Bond Future
    register_pricer!(
        registry,
        BondFuture,
        Discounting,
        crate::instruments::fixed_income::bond_future::pricer::BondFuturePricer
    );

    // Cap/Floor
    register_pricer!(
        registry,
        CapFloor,
        Black76,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default()
    );
    register_pricer!(
        registry,
        CapFloor,
        Discounting,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Swaption
    register_pricer!(
        registry,
        Swaption,
        Black76,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        Swaption,
        Discounting,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Repo
    register_pricer!(
        registry,
        Repo,
        Discounting,
        crate::instruments::rates::repo::pricer::SimpleRepoDiscountingPricer::default()
    );

    // DCF (Discounted Cash Flow)
    register_pricer!(
        registry,
        DCF,
        Discounting,
        crate::instruments::equity::dcf_equity::pricer::DcfPricer
    );
}

/// Register pricers for credit instruments.
pub fn register_credit_pricers(registry: &mut PricerRegistry) {
    // CDS
    register_pricer!(
        registry,
        CDS,
        HazardRate,
        crate::instruments::common_impl::GenericInstrumentPricer::cds()
    );
    register_pricer!(
        registry,
        CDS,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CreditDefaultSwap,
        >::new(InstrumentType::CDS, ModelKey::Discounting)
    );

    // CDS Index
    register_pricer!(
        registry,
        CDSIndex,
        HazardRate,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSIndex,
        Discounting,
        crate::instruments::credit_derivatives::cds_index::pricer::SimpleCdsIndexHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Tranche
    register_pricer!(
        registry,
        CDSTranche,
        HazardRate,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::default()
    );
    register_pricer!(
        registry,
        CDSTranche,
        Discounting,
        crate::instruments::credit_derivatives::cds_tranche::pricer::SimpleCdsTrancheHazardPricer::with_model(
            ModelKey::Discounting
        )
    );

    // CDS Option
    register_pricer!(
        registry,
        CDSOption,
        Black76,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCdsOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CDSOption,
        Discounting,
        crate::instruments::credit_derivatives::cds_option::pricer::SimpleCdsOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Structured Credit - unified pricer for ABS, CLO, CMBS, RMBS
    register_pricer!(
        registry,
        StructuredCredit,
        Discounting,
        crate::instruments::fixed_income::structured_credit::StructuredCreditDiscountingPricer::default()
    );
}

/// Register pricers for equity instruments.
pub fn register_equity_pricers(registry: &mut PricerRegistry) {
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
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        EquityOption,
        Discounting,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        EquityOption,
        HestonFourier,
        crate::instruments::equity::equity_option::pricer::EquityOptionHestonFourierPricer
    );

    // Equity TRS
    register_pricer!(
        registry,
        EquityTotalReturnSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::equity::equity_trs::EquityTotalReturnSwap,
        >::discounting(InstrumentType::EquityTotalReturnSwap)
    );

    // Variance Swap
    register_pricer!(
        registry,
        VarianceSwap,
        Discounting,
        crate::instruments::equity::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default()
    );

    // Equity Index Future - uses GenericInstrumentPricer
    register_pricer!(
        registry,
        EquityIndexFuture,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::EquityIndexFuture,
        >::discounting(InstrumentType::EquityIndexFuture)
    );

    // Real Estate Asset - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        RealEstateAsset,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::RealEstateAsset,
        >::discounting(InstrumentType::RealEstateAsset)
    );
}

/// Register pricers for FX instruments.
pub fn register_fx_pricers(registry: &mut PricerRegistry) {
    // FX Spot
    register_pricer!(
        registry,
        FxSpot,
        Discounting,
        crate::instruments::fx::fx_spot::pricer::FxSpotPricer
    );

    // FX Swap
    register_pricer!(
        registry,
        FxSwap,
        Discounting,
        crate::instruments::fx::fx_swap::pricer::SimpleFxSwapDiscountingPricer::default()
    );

    // XCCY Swap
    register_pricer!(
        registry,
        XccySwap,
        Discounting,
        crate::instruments::rates::xccy_swap::pricer::SimpleXccySwapDiscountingPricer::default()
    );

    // FX Option
    register_pricer!(
        registry,
        FxOption,
        Black76,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        FxOption,
        Discounting,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // FX Variance Swap
    register_pricer!(
        registry,
        FxVarianceSwap,
        Discounting,
        crate::instruments::fx::fx_variance_swap::pricer::SimpleFxVarianceSwapDiscountingPricer::default()
    );

    // FX Forward - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        FxForward,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::FxForward,
        >::discounting(InstrumentType::FxForward)
    );

    // NDF (Non-Deliverable Forward) - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        Ndf,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<crate::instruments::Ndf>::discounting(
            InstrumentType::Ndf
        )
    );

    // FX Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        FxBarrierOption,
        MonteCarloGBM,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionMcPricer::default()
    );
    register_pricer!(
        registry,
        FxBarrierOption,
        FxBarrierBSContinuous,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer
    );

    // FX Digital Option
    register_pricer!(
        registry,
        FxDigitalOption,
        Black76,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::default()
    );
    register_pricer!(
        registry,
        FxDigitalOption,
        Discounting,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::with_model(
            ModelKey::Discounting
        )
    );

    // FX Touch Option
    register_pricer!(
        registry,
        FxTouchOption,
        Black76,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::default()
    );
    register_pricer!(
        registry,
        FxTouchOption,
        Discounting,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::with_model(
            ModelKey::Discounting
        )
    );
}

/// Create a pricer registry with a minimal set of rates pricers.
pub fn create_rates_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_rates_pricers(&mut registry);
    registry
}

/// Create a pricer registry with credit pricers.
pub fn create_credit_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_credit_pricers(&mut registry);
    registry
}

/// Create a pricer registry with equity pricers.
pub fn create_equity_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_equity_pricers(&mut registry);
    registry
}

/// Create a pricer registry with FX pricers.
pub fn create_fx_registry() -> PricerRegistry {
    let mut registry = PricerRegistry::new();
    register_fx_pricers(&mut registry);
    registry
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
    fn pricing_error_maps_to_structured_core_errors() {
        // MissingMarketData -> InputError::NotFound
        let missing: finstack_core::Error = PricingError::MissingMarketData {
            missing_id: "USD-SOFR".to_string(),
            context: PricingErrorContext::default(),
        }
        .into();
        match missing {
            finstack_core::Error::Input(finstack_core::InputError::NotFound { id }) => {
                assert_eq!(id, "USD-SOFR")
            }
            other => panic!("unexpected mapping for missing market data: {other:?}"),
        }

        // UnknownPricer -> InputError::NotFound
        let unknown_pricer: finstack_core::Error =
            PricingError::UnknownPricer(PricerKey::new(InstrumentType::Bond, ModelKey::Tree))
                .into();
        match unknown_pricer {
            finstack_core::Error::Input(finstack_core::InputError::NotFound { id }) => {
                assert_eq!(id, "pricer:Bond:Tree")
            }
            other => panic!("unexpected mapping for unknown pricer: {other:?}"),
        }

        // TypeMismatch -> InputError::Invalid
        let type_mismatch: finstack_core::Error = PricingError::TypeMismatch {
            expected: InstrumentType::Bond,
            got: InstrumentType::IRS,
        }
        .into();
        match type_mismatch {
            finstack_core::Error::Input(finstack_core::InputError::Invalid) => {}
            other => panic!("unexpected mapping for type mismatch: {other:?}"),
        }

        // InvalidInput -> Error::Validation (not Calibration)
        let invalid_input: finstack_core::Error = PricingError::InvalidInput {
            message: "bad parameter".to_string(),
            context: PricingErrorContext::new().instrument_id("TEST-001"),
        }
        .into();
        match invalid_input {
            finstack_core::Error::Validation(msg) => {
                assert!(
                    msg.contains("bad parameter"),
                    "Validation message should contain original message"
                );
                assert!(
                    msg.contains("TEST-001"),
                    "Validation message should contain context"
                );
            }
            other => panic!("unexpected mapping for invalid input: {other:?}"),
        }

        // ModelFailure -> Calibration (for numerical/solver failures)
        let model_failure: finstack_core::Error = PricingError::ModelFailure {
            message: "solver did not converge".to_string(),
            context: PricingErrorContext::default(),
        }
        .into();
        match model_failure {
            finstack_core::Error::Calibration { category, message } => {
                assert_eq!(category, "pricing_model");
                assert!(message.contains("solver did not converge"));
            }
            other => panic!("unexpected mapping for model failure: {other:?}"),
        }
    }

    #[test]
    fn core_error_maps_to_pricing_error_categories() {
        // Input::NotFound -> MissingMarketData
        let core_missing: finstack_core::Error = finstack_core::InputError::NotFound {
            id: "USD-OIS".to_string(),
        }
        .into();
        let pricing: PricingError = core_missing.into();
        match pricing {
            PricingError::MissingMarketData { missing_id, .. } => assert_eq!(missing_id, "USD-OIS"),
            other => panic!("unexpected mapping for missing input: {other:?}"),
        }

        // Validation -> InvalidInput (not ModelFailure)
        let core_invalid = finstack_core::Error::Validation("bad parameter".to_string());
        let pricing: PricingError = core_invalid.into();
        match pricing {
            PricingError::InvalidInput { message, .. } => {
                assert!(message.contains("bad parameter"));
            }
            other => panic!("unexpected mapping for validation: {other:?}"),
        }

        // Calibration -> ModelFailure
        let core_calibration = finstack_core::Error::Calibration {
            message: "solver did not converge".to_string(),
            category: "solver".to_string(),
        };
        let pricing: PricingError = core_calibration.into();
        match pricing {
            PricingError::ModelFailure { message, .. } => {
                assert!(message.contains("solver did not converge"));
            }
            other => panic!("unexpected mapping for calibration: {other:?}"),
        }
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

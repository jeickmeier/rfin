//! Strongly-typed keys for pricer dispatch.
//!
//! Defines [`InstrumentType`], [`ModelKey`], and [`PricerKey`] — the enum-based
//! identifiers used by the pricing registry to route instruments to the
//! appropriate pricer implementation.

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::EnumIter,
)]
#[repr(u16)]
/// Strongly-typed instrument classification for pricer dispatch.
///
/// Each variant represents a distinct instrument type with its own pricing
/// logic and risk characteristics. Used by the pricing registry to route
/// instruments to appropriate pricer implementations.
#[non_exhaustive]
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
    /// Option on interest rate future (e.g., SOFR futures option).
    IrFutureOption = 76,
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
    /// CMS swap (constant maturity swap).
    CmsSwap = 77,
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
    /// Levered real estate equity (asset minus financing).
    LeveredRealEstateEquity = 73,
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
    /// Commodity swaption (option on a fixed-for-floating commodity swap).
    CommoditySwaption = 74,
    /// Commodity spread option (option on the spread between two commodities).
    CommoditySpreadOption = 75,
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
            InstrumentType::IrFutureOption => "ir_future_option",
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
            InstrumentType::CmsSwap => "cms_swap",
            InstrumentType::CliquetOption => "cliquet_option",
            InstrumentType::RangeAccrual => "range_accrual",
            InstrumentType::FxBarrierOption => "fx_barrier_option",
            InstrumentType::TermLoan => "term_loan",
            InstrumentType::DCF => "dcf",
            InstrumentType::RealEstateAsset => "real_estate_asset",
            InstrumentType::LeveredRealEstateEquity => "levered_real_estate_equity",
            InstrumentType::EquityTotalReturnSwap => "equity_total_return_swap",
            InstrumentType::FIIndexTotalReturnSwap => "fi_index_total_return_swap",
            InstrumentType::BondFuture => "bond_future",
            InstrumentType::CommodityForward => "commodity_forward",
            InstrumentType::CommoditySwap => "commodity_swap",
            InstrumentType::CommodityOption => "commodity_option",
            InstrumentType::CommodityAsianOption => "commodity_asian_option",
            InstrumentType::CommoditySwaption => "commodity_swaption",
            InstrumentType::CommoditySpreadOption => "commodity_spread_option",
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
            "ir_future_option" | "irfutureoption" => Ok(InstrumentType::IrFutureOption),
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
            "cms_swap" => Ok(InstrumentType::CmsSwap),
            "cliquet_option" | "cliquet" => Ok(InstrumentType::CliquetOption),
            "range_accrual" | "range_accrual_note" => Ok(InstrumentType::RangeAccrual),
            "fx_barrier_option" | "fx_barrier" => Ok(InstrumentType::FxBarrierOption),
            "term_loan" | "termloan" | "loan_term" => Ok(InstrumentType::TermLoan),
            "dcf" | "discounted_cash_flow" => Ok(InstrumentType::DCF),
            "real_estate_asset" | "real_estate" | "realestate" | "realestate_asset" => {
                Ok(InstrumentType::RealEstateAsset)
            }
            "levered_real_estate_equity"
            | "levered_real_estate"
            | "levered_re_equity"
            | "real_estate_equity_levered" => Ok(InstrumentType::LeveredRealEstateEquity),
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
            "commodity_asian_option" | "commodityasianoption" => {
                Ok(InstrumentType::CommodityAsianOption)
            }
            "commodity_swaption" | "commodityswaption" => Ok(InstrumentType::CommoditySwaption),
            "commodity_spread_option" | "commodityspreadoption" => {
                Ok(InstrumentType::CommoditySpreadOption)
            }
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::EnumIter,
)]
#[non_exhaustive]
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
    /// Merton structural credit Monte Carlo with PIK support.
    ///
    /// Used for: PIK bonds, credit-risky bonds with structural default model.
    MertonMc = 30,
    /// Monte Carlo with Schwartz-Smith two-factor commodity model.
    ///
    /// Used for: commodity options requiring mean-reverting short-term dynamics.
    MonteCarloSchwartzSmith = 31,
    /// Static replication via a portfolio of vanilla options.
    ///
    /// Used for: CMS options (replicates the payoff with a smile-consistent
    /// swaption portfolio, capturing all convexity orders beyond Hagan's
    /// first-order approximation).
    StaticReplication = 32,
    /// LMM/BGM Monte Carlo with predictor-corrector discretization.
    ///
    /// Used for: Bermudan swaptions, exotic rate derivatives requiring
    /// multi-factor forward rate dynamics.
    LmmMonteCarlo = 33,
}

impl ModelKey {
    /// Returns true when the model is only available in builds with the `mc` feature.
    pub const fn requires_mc_feature(self) -> bool {
        matches!(
            self,
            Self::MonteCarloGBM
                | Self::MonteCarloHeston
                | Self::MonteCarloHullWhite1F
                | Self::HestonFourier
                | Self::MertonMc
                | Self::MonteCarloSchwartzSmith
                | Self::LmmMonteCarlo
        )
    }
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
            ModelKey::MertonMc => "merton_mc",
            ModelKey::MonteCarloSchwartzSmith => "monte_carlo_schwartz_smith",
            ModelKey::StaticReplication => "static_replication",
            ModelKey::LmmMonteCarlo => "lmm_monte_carlo",
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
            "merton_mc" | "merton" | "structural_mc" => Ok(ModelKey::MertonMc),
            "monte_carlo_schwartz_smith" | "mc_schwartz_smith" | "schwartz_smith_mc" => {
                Ok(ModelKey::MonteCarloSchwartzSmith)
            }
            "static_replication" | "static_rep" | "replication" => Ok(ModelKey::StaticReplication),
            "lmm_monte_carlo" | "lmm_mc" | "lmm" | "bgm" | "bgm_mc" => Ok(ModelKey::LmmMonteCarlo),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn abi_is_stable() {
        use core::mem::size_of;
        assert_eq!(size_of::<InstrumentType>(), 2);
        assert_eq!(size_of::<ModelKey>(), 2);
        assert_eq!(size_of::<PricerKey>(), 4);
    }

    #[test]
    fn test_instrument_type_bond_future() {
        let inst_type = InstrumentType::BondFuture;
        assert_eq!(format!("{}", inst_type), "bond_future");
        assert_eq!(inst_type as u16, 54);
    }

    #[test]
    fn instrument_type_display_from_str_roundtrip() {
        for variant in InstrumentType::iter() {
            let s = variant.to_string();
            let parsed: InstrumentType = s
                .parse()
                .unwrap_or_else(|e| panic!("{variant:?} Display=\"{s}\" failed FromStr: {e}"));
            assert_eq!(parsed, variant, "round-trip failed for {variant:?}");
        }
    }

    #[test]
    fn model_key_display_from_str_roundtrip() {
        for variant in ModelKey::iter() {
            let s = variant.to_string();
            let parsed: ModelKey = s
                .parse()
                .unwrap_or_else(|e| panic!("{variant:?} Display=\"{s}\" failed FromStr: {e}"));
            assert_eq!(parsed, variant, "round-trip failed for {variant:?}");
        }
    }
}

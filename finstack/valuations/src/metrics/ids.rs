//! Strongly-typed metric identifiers for compile-time validation.
//!
//! Provides a comprehensive set of metric IDs covering bond, IRS, deposit,
//! and risk metrics. Each ID is strongly-typed to prevent runtime errors
//! and enable compile-time validation of metric dependencies.
//!
//! # Metric Categories
//!
//! - **Bond metrics**: Yield, duration, convexity, pricing, credit spreads
//! - **IRS metrics**: DV01, annuity factors, par rates, present values
//! - **Deposit metrics**: Discount factors, par rates, year fractions
//! - **Risk metrics**: Bucketed DV01, time decay (theta)
//! - **Custom metrics**: User-defined metrics with dynamic identifiers

use std::fmt;
use std::str::FromStr;

macro_rules! define_metrics {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $str:literal
        ),+ $(,)?
    ) => {
        /// Strongly-typed metric identifier.
        ///
        /// Provides compile-time validation, autocomplete support, and safe refactoring
        /// when metric names change. Covers bond, IRS, deposit, and risk metrics.
        ///
        /// See unit tests and `examples/` for usage.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum MetricId {
            $(
                $(#[$meta])*
                $variant,
            )+
            /// Custom metric with a dynamic identifier
            Custom(String),
        }

        impl MetricId {
            /// Creates a custom metric ID.
            ///
            /// Use this for user-defined metrics that aren't part of the standard set.
            /// Custom metrics are stored as strings and can have any identifier.
            pub fn custom(id: impl Into<String>) -> Self {
                MetricId::Custom(id.into())
            }

            /// Converts to string representation for compatibility.
            ///
            /// Returns a lowercase, snake_case string that can be used for
            /// serialization, logging, or API interfaces.
            pub fn as_str(&self) -> &str {
                match self {
                    $(
                        MetricId::$variant => $str,
                    )+
                    MetricId::Custom(s) => s.as_str(),
                }
            }

            /// All standard (non-custom) metric IDs.
            pub const ALL_STANDARD: &'static [MetricId] = &[
                $(
                    MetricId::$variant,
                )+
            ];
        }

        impl fmt::Display for MetricId {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl FromStr for MetricId {
            type Err = (); // Never fails since we have a catch-all Custom variant

            /// Parses a string into a MetricId.
            ///
            /// This method never fails - any unrecognized string becomes a custom metric.
            /// Standard metrics are matched case-insensitively in snake_case format.
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let lower = s.to_lowercase();
                let metric_id = match lower.as_str() {
                    $(
                        $str => MetricId::$variant,
                    )+
                    _ => MetricId::Custom(lower),
                };
                Ok(metric_id)
            }
        }
    };
}

define_metrics! {

    // FX spot metrics
    /// Spot rate
    SpotRate => "spot_rate",
    /// Base amount
    BaseAmount => "base_amount",
    /// Quote amount
    QuoteAmount => "quote_amount",
    /// Inverse rate
    InverseRate => "inverse_rate",

    // Equity metrics
    /// Equity price per share
    EquityPricePerShare => "equity_price_per_share",
    /// Equity shares (effective)
    EquityShares => "equity_shares",
    /// Equity dividend yield (annualized, decimal)
    EquityDividendYield => "equity_dividend_yield",
    /// Equity forward price per share
    EquityForwardPrice => "equity_forward_price",

    // Bond metrics
    /// Dirty price (includes accrued interest)
    DirtyPrice => "dirty_price",
    /// Clean price (excludes accrued interest)
    CleanPrice => "clean_price",
    /// Accrued interest since last coupon payment
    Accrued => "accrued",
    /// Accrued interest (alias of `accrued` for compatibility)
    AccruedInterest => "accrued_interest",
    /// Yield to maturity
    Ytm => "ytm",
    /// Yield to worst
    Ytw => "ytw",
    /// Macaulay duration
    DurationMac => "duration_mac",
    /// Modified duration
    DurationMod => "duration_mod",
    /// Convexity
    Convexity => "convexity",

    // Spread metrics
    /// Z-spread - Zero-vol spread
    ZSpread => "z_spread",
    /// OAS - Option-adjusted spread
    Oas => "oas",
    /// I-spread - Yield over interpolated swap curve
    ISpread => "i_spread",
    /// Discount margin for floating-rate bonds (decimal; 0.01 = 100 bps)
    DiscountMargin => "discount_margin",
    /// G-spread - Govvie spread
    GSpread => "g_spread",
    /// ASW-spread - Asset swap spread (par)
    ASWSpread => "asw_spread",
    /// Par asset swap spread
    ASWPar => "asw_par",
    /// Market (price) asset swap spread
    ASWMarket => "asw_market",
    /// Par asset swap spread using forward curve (requires BondFloatSpec or explicit forward)
    ASWParFwd => "asw_par_fwd",
    /// Market asset swap spread using forward curve (requires BondFloatSpec or explicit forward)
    ASWMarketFwd => "asw_market_fwd",

    // IRS metrics
    /// Annuity factor for fixed leg
    Annuity => "annuity",
    /// Par swap rate
    ParRate => "par_rate",
    /// Dollar value of 01 (DV01)
    Dv01 => "dv01",
    /// Present value of 01 (PV01) - Alias for DV01 (credit market convention)
    Pv01 => "pv01",
    /// Present value of fixed leg
    PvFixed => "pv_fixed",
    /// Present value of floating leg
    PvFloat => "pv_float",

    // Deposit metrics
    /// Year fraction
    Yf => "yf",
    /// Discount factor at start date
    DfStart => "df_start",
    /// Discount factor at end date
    DfEnd => "df_end",
    /// Deposit par rate
    DepositParRate => "deposit_par_rate",
    /// Discount factor at end date from quote
    DfEndFromQuote => "df_end_from_quote",
    /// Quote rate
    QuoteRate => "quote_rate",

    // CDS metrics
    /// Par spread for CDS
    ParSpread => "par_spread",
    /// Risky PV01 for CDS
    RiskyPv01 => "risky_pv01",
    /// Protection leg present value
    ProtectionLegPv => "protection_leg_pv",
    /// Premium leg present value
    PremiumLegPv => "premium_leg_pv",
    /// Jump-to-default amount
    JumpToDefault => "jump_to_default",
    /// Expected loss
    ExpectedLoss => "expected_loss",
    /// Default probability
    DefaultProbability => "default_probability",
    /// Expected recovery rate
    Recovery01 => "recovery_01",

    // Option metrics
    /// Delta (price sensitivity to underlying)
    Delta => "delta",
    /// Gamma (delta sensitivity to underlying)
    Gamma => "gamma",
    /// Vega (price sensitivity to volatility)
    Vega => "vega",
    /// Rho (price sensitivity to interest rates)
    Rho => "rho",
    /// Forward curve PV01 (price sensitivity to a 1bp forward curve bump)
    ForwardPv01 => "forward_pv01",
    /// Vanna (delta sensitivity to volatility)
    Vanna => "vanna",
    /// Volga (vega sensitivity to volatility)
    Volga => "volga",
    /// Veta (theta sensitivity to volatility)
    Veta => "veta",
    /// Charm (rho sensitivity to volatility)
    Charm => "charm",
    /// Color (gamma sensitivity to time)
    Color => "color",
    /// Speed (gamma sensitivity to underlying)
    Speed => "speed",
    /// Implied volatility (from price)
    ImpliedVol => "implied_vol",

    // Variance swap metrics
    /// Vega expressed per variance point (variance swap sensitivity)
    VarianceVega => "variance_vega",
    /// Expected variance under the pricing model
    ExpectedVariance => "variance_expected",
    /// Realized variance computed from observed paths
    RealizedVariance => "variance_realized",
    /// Variance notional exposure (payout multiplier)
    VarianceNotional => "variance_notional",
    /// Strike volatility equivalent (sqrt of strike variance)
    VarianceStrikeVol => "variance_strike_vol",
    /// Time to maturity as used in the variance swap conventions
    VarianceTimeToMaturity => "variance_time_to_maturity",

    // Risk metrics
    /// Credit spread sensitivity (CS01) - Parallel shift in credit spread
    Cs01 => "cs01",
    /// Hazard curve sensitivity (CS01) - Parallel additive hazard rate bump
    HazardCs01 => "hazard_cs01",
    /// IR01 - Parallel shift in yield curve
    Ir01 => "ir01",
    /// Bucketed DV01 risk - Pointwise sensitivity to yield curve
    BucketedDv01 => "bucketed_dv01",
    /// Bucketed Credit Spread Risk - Pointwise sensitivity to credit spread
    BucketedCs01 => "bucketed_cs01",
    /// Time decay (theta) - 1D Day Time decay P&L
    Theta => "theta",

    // TRS metrics
    /// Financing annuity for TRS
    FinancingAnnuity => "financing_annuity",
    /// Index delta for TRS
    IndexDelta => "index_delta",

    // Basis swap metrics (using consistent leg naming with IRS)
    /// PV of primary floating leg (includes spread)
    PvPrimary => "pv_primary",
    /// PV of reference floating leg
    PvReference => "pv_reference",
    /// Annuity of primary leg
    AnnuityPrimary => "annuity_primary",
    /// Annuity of reference leg
    AnnuityReference => "annuity_reference",
    /// DV01 of primary leg
    Dv01Primary => "dv01_primary",
    /// DV01 of reference leg
    Dv01Reference => "dv01_reference",
    /// Par spread for basis swap
    BasisParSpread => "basis_par_spread",

    // Repo metrics
    /// Market value of collateral
    CollateralValue => "collateral_value",
    /// Required collateral value (with haircut)
    RequiredCollateral => "required_collateral",
    /// Collateral coverage ratio
    CollateralCoverage => "collateral_coverage",
    /// Repo interest amount
    RepoInterest => "repo_interest",
    /// Funding risk (repo rate sensitivity)
    FundingRisk => "funding_risk",
    /// Effective repo rate (adjusted for special collateral)
    EffectiveRate => "effective_rate",
    /// Time to maturity in years
    TimeToMaturity => "time_to_maturity",
    /// Implied collateral return
    ImpliedCollateralReturn => "implied_collateral_return",

    // Basket/ETF metrics
    /// Net Asset Value per share
    Nav => "nav",
    /// Total basket value
    BasketValue => "basket_value",
    /// Number of constituents in the basket
    ConstituentCount => "constituent_count",
    /// Expense ratio as percentage
    ExpenseRatio => "expense_ratio",
    /// Tracking error vs benchmark
    TrackingError => "tracking_error",
    /// Utilization vs creation unit size
    Utilization => "utilization",
    /// Premium/discount to NAV
    PremiumDiscount => "premium_discount",

    // === Structured Credit Metrics ===

    /// Weighted Average Life (years) - Expected principal repayment life
    WAL => "wal",

    /// Weighted Average Maturity (years) - Pool maturity
    WAM => "wam",

    /// Expected final payment date under base assumptions
    ExpectedMaturity => "expected_maturity",

    /// Percentage of original pool balance remaining
    PoolFactor => "pool_factor",

    /// Constant Prepayment Rate (annualized)
    CPR => "cpr",

    /// Single Monthly Mortality (monthly prepayment rate)
    SMM => "smm",

    /// Constant Default Rate (annualized)
    CDR => "cdr",

    /// Loss Severity (1 - Recovery Rate)
    LossSeverity => "loss_severity",

    /// Spread duration (time-weighted sensitivity to spread changes)
    SpreadDuration => "spread_duration",

    /// DM01 - Discount margin sensitivity (for floating-rate CLO)
    Dm01 => "dm01",

    // === ABS-specific Metrics ===

    /// Delinquency rate - Percentage of pool in delinquency
    AbsDelinquency => "abs_delinquency",

    /// Charge-off rate - Percentage of pool charged off
    AbsChargeOff => "abs_charge_off",

    /// Excess spread - Spread available to absorb losses
    AbsExcessSpread => "abs_excess_spread",

    /// Credit enhancement level - Subordination as % of pool
    AbsCreditEnhancement => "abs_ce_level",

    // === CLO-specific Metrics ===

    /// Weighted Average Rating Factor
    CloWarf => "clo_warf",

    /// Weighted Average Spread
    CloWas => "clo_was",

    /// Weighted Average Coupon
    CloWac => "clo_wac",

    /// Portfolio diversity score
    CloDiversity => "clo_diversity",

    /// Overcollateralization ratio
    CloOcRatio => "clo_oc_ratio",

    /// Interest coverage ratio
    CloIcRatio => "clo_ic_ratio",

    /// Average recovery rate on defaults
    CloRecoveryRate => "clo_recovery_rate",

    // === CMBS-specific Metrics ===

    /// Debt Service Coverage Ratio
    CmbsDscr => "cmbs_dscr",

    /// Weighted Average Loan-to-Value
    CmbsWaltv => "cmbs_waltv",

    /// Credit Enhancement Level
    CmbsCreditEnhancement => "cmbs_ce_level",

    // === RMBS-specific Metrics ===

    /// PSA prepayment speed (e.g., 100% PSA)
    RmbsPsaSpeed => "rmbs_psa_speed",

    /// SDA default speed
    RmbsSdaSpeed => "rmbs_sda_speed",

    /// Weighted Average LTV for RMBS
    RmbsWaltv => "rmbs_waltv",

    /// Weighted Average FICO score
    RmbsWafico => "rmbs_wafico",

    // === Inflation-Linked Bond Metrics ===

    /// Real yield (inflation-adjusted)
    RealYield => "real_yield",

    /// Inflation index ratio
    IndexRatio => "index_ratio",

    /// Real duration (inflation-adjusted duration)
    RealDuration => "real_duration",

    /// Breakeven inflation rate
    BreakevenInflation => "breakeven_inflation",

    // === Private Equity / Private Markets Fund Metrics ===

    /// LP (Limited Partner) Internal Rate of Return
    LpIrr => "lp_irr",

    /// GP (General Partner) Internal Rate of Return
    GpIrr => "gp_irr",

    /// LP Multiple on Invested Capital
    MoicLp => "moic_lp",

    /// LP Distributions to Paid In (DPI ratio)
    DpiLp => "dpi_lp",

    /// LP Total Value to Paid In (TVPI ratio)
    TvpiLp => "tvpi_lp",

    /// Accrued carry amount for GP
    CarryAccrued => "carry_accrued",

}

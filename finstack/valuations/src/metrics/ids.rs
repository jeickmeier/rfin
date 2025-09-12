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
                let metric_id = match s.to_lowercase().as_str() {
                    $(
                        $str => MetricId::$variant,
                    )+
                    s => MetricId::Custom(s.to_string()),
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

    // Bond metrics
    /// Dirty price (includes accrued interest)
    DirtyPrice => "dirty_price",
    /// Clean price (excludes accrued interest)
    CleanPrice => "clean_price",
    /// Accrued interest since last coupon payment
    Accrued => "accrued",
    /// Yield to maturity
    Ytm => "ytm",
    /// Yield to worst
    Ytw => "ytw",
    /// Macaulay duration
    DurationMac => "duration_mac",
    /// Modified duration
    DurationMod => "duration_mod",
    /// Credit duration
    CreditDuration => "credit_duration",
    /// Convexity
    Convexity => "convexity",

    // Spread metrics
    /// Z-spread - Zero-vol spread
    ZSpread => "z_spread",
    /// OAS - Option-adjusted spread
    Oas => "oas",
    /// G-spread - Govvie spread
    GSpread => "g_spread",
    /// ASW-spread - Asset swap spread
    ASWSpread => "asw_spread",

    // IRS metrics
    /// Annuity factor for fixed leg
    Annuity => "annuity",
    /// Par swap rate
    ParRate => "par_rate",
    /// Dollar value of 01 (DV01)
    Dv01 => "dv01",
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

    // Risk metrics
    /// Credit spread sensitivity (CS01) - Parallel shift in credit spread
    Cs01 => "cs01",
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

}

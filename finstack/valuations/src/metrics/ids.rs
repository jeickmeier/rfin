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

/// Strongly-typed metric identifier.
/// 
/// Provides compile-time validation, autocomplete support, and safe refactoring
/// when metric names change. Covers bond, IRS, deposit, and risk metrics.
/// 
/// # Examples
/// 
/// ```rust
/// use finstack_valuations::metrics::ids::MetricId;
/// 
/// // Standard metrics
/// let ytm = MetricId::Ytm;
/// let dv01 = MetricId::Dv01;
/// 
/// // Custom metrics
/// let custom = MetricId::custom("my_metric");
/// 
/// // String conversion
/// assert_eq!(ytm.as_str(), "ytm");
/// assert_eq!(dv01.as_str(), "dv01");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricId {
    // Bond metrics
    /// Accrued interest since last coupon payment
    Accrued,
    /// Yield to maturity
    Ytm,
    /// Macaulay duration
    DurationMac,
    /// Modified duration
    DurationMod,
    /// Convexity
    Convexity,
    /// Yield to worst
    Ytw,
    /// Dirty price (includes accrued interest)
    DirtyPrice,
    /// Clean price (excludes accrued interest)
    CleanPrice,
    /// Credit spread sensitivity (CS01)
    Cs01,
    
    // IRS metrics
    /// Annuity factor for fixed leg
    Annuity,
    /// Par swap rate
    ParRate,
    /// Dollar value of 01 (DV01)
    Dv01,
    /// Present value of fixed leg
    PvFixed,
    /// Present value of floating leg
    PvFloat,
    
    // Deposit metrics
    /// Year fraction
    Yf,
    /// Discount factor at start date
    DfStart,
    /// Discount factor at end date
    DfEnd,
    /// Deposit par rate
    DepositParRate,
    /// Discount factor at end date from quote
    DfEndFromQuote,
    /// Quote rate
    QuoteRate,
    
    // Risk metrics
    /// Bucketed DV01 risk
    BucketedDv01,
    /// Time decay (theta)
    Theta,
    
    // CDS metrics
    /// Par spread for CDS
    ParSpread,
    /// Risky PV01 for CDS
    RiskyPv01,
    /// Protection leg present value
    ProtectionLegPv,
    /// Premium leg present value
    PremiumLegPv,
    
    // Option metrics
    /// Delta (price sensitivity to underlying)
    Delta,
    /// Gamma (delta sensitivity to underlying)
    Gamma,
    /// Vega (price sensitivity to volatility)
    Vega,
    /// Rho (price sensitivity to interest rates)
    Rho,
    
    // Custom metrics
    /// Custom metric with a dynamic identifier
    Custom(String),
}

impl MetricId {
    /// Creates a custom metric ID.
    /// 
    /// Use this for user-defined metrics that aren't part of the standard set.
    /// Custom metrics are stored as strings and can have any identifier.
    /// 
    /// # Arguments
    /// * `id` - String identifier for the custom metric
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::metrics::ids::MetricId;
    /// 
    /// let custom = MetricId::custom("my_metric");
    /// assert!(matches!(custom, MetricId::Custom(_)));
    /// assert_eq!(custom.as_str(), "my_metric");
    /// ```
    pub fn custom(id: impl Into<String>) -> Self {
        MetricId::Custom(id.into())
    }
    
    /// Converts to string representation for compatibility.
    /// 
    /// Returns a lowercase, snake_case string that can be used for
    /// serialization, logging, or API interfaces.
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::metrics::ids::MetricId;
    /// 
    /// assert_eq!(MetricId::Ytm.as_str(), "ytm");
    /// assert_eq!(MetricId::Dv01.as_str(), "dv01");
    /// assert_eq!(MetricId::DurationMac.as_str(), "duration_mac");
    /// ```
    pub fn as_str(&self) -> &str {
        match self {
            // Bond metrics
            MetricId::Accrued => "accrued",
            MetricId::Ytm => "ytm",
            MetricId::DurationMac => "duration_mac",
            MetricId::DurationMod => "duration_mod",
            MetricId::Convexity => "convexity",
            MetricId::Ytw => "ytw",
            MetricId::DirtyPrice => "dirty_price",
            MetricId::CleanPrice => "clean_price",
            MetricId::Cs01 => "cs01",
            
            // IRS metrics
            MetricId::Annuity => "annuity",
            MetricId::ParRate => "par_rate",
            MetricId::Dv01 => "dv01",
            MetricId::PvFixed => "pv_fixed",
            MetricId::PvFloat => "pv_float",
            
            // Deposit metrics
            MetricId::Yf => "yf",
            MetricId::DfStart => "df_start",
            MetricId::DfEnd => "df_end",
            MetricId::DepositParRate => "deposit_par_rate",
            MetricId::DfEndFromQuote => "df_end_from_quote",
            MetricId::QuoteRate => "quote_rate",
            
            // Risk metrics
            MetricId::BucketedDv01 => "bucketed_dv01",
            MetricId::Theta => "theta",
            
            // CDS metrics
            MetricId::ParSpread => "par_spread",
            MetricId::RiskyPv01 => "risky_pv01",
            MetricId::ProtectionLegPv => "protection_leg_pv",
            MetricId::PremiumLegPv => "premium_leg_pv",
            
            // Option metrics
            MetricId::Delta => "delta",
            MetricId::Gamma => "gamma",
            MetricId::Vega => "vega",
            MetricId::Rho => "rho",
            
            // Custom metrics
            MetricId::Custom(s) => s.as_str(),
        }
    }
    
    /// All standard (non-custom) metric IDs.
    /// 
    /// This constant provides access to all predefined metrics for
    /// iteration, validation, or registry initialization.
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::metrics::ids::MetricId;
    /// 
    /// let standard_count = MetricId::ALL_STANDARD.len();
    /// assert!(standard_count > 20); // Should have many standard metrics
    /// 
    /// // Check if a metric is standard
    /// assert!(MetricId::ALL_STANDARD.contains(&MetricId::Ytm));
    /// assert!(!MetricId::ALL_STANDARD.contains(&MetricId::custom("custom")));
    /// ```
    pub const ALL_STANDARD: &'static [MetricId] = &[
        // Bond metrics
        MetricId::Accrued,
        MetricId::Ytm,
        MetricId::DurationMac,
        MetricId::DurationMod,
        MetricId::Convexity,
        MetricId::Ytw,
        MetricId::DirtyPrice,
        MetricId::CleanPrice,
        MetricId::Cs01,
        
        // IRS metrics
        MetricId::Annuity,
        MetricId::ParRate,
        MetricId::Dv01,
        MetricId::PvFixed,
        MetricId::PvFloat,
        
        // Deposit metrics
        MetricId::Yf,
        MetricId::DfStart,
        MetricId::DfEnd,
        MetricId::DepositParRate,
        MetricId::DfEndFromQuote,
        MetricId::QuoteRate,
        
        // Risk metrics
        MetricId::BucketedDv01,
        MetricId::Theta,
        
        // CDS metrics
        MetricId::ParSpread,
        MetricId::RiskyPv01,
        MetricId::ProtectionLegPv,
        MetricId::PremiumLegPv,
        
        // Option metrics
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Rho,
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
    /// 
    /// # Example
    /// ```rust
    /// use finstack_valuations::metrics::ids::MetricId;
    /// use std::str::FromStr;
    /// 
    /// // Standard metrics
    /// assert_eq!(MetricId::from_str("ytm").unwrap(), MetricId::Ytm);
    /// assert_eq!(MetricId::from_str("DV01").unwrap(), MetricId::Dv01);
    /// 
    /// // Custom metrics
    /// let custom = MetricId::from_str("my_metric").unwrap();
    /// assert!(matches!(custom, MetricId::Custom(_)));
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let metric_id = match s.to_lowercase().as_str() {
            // Bond metrics
            "accrued" => MetricId::Accrued,
            "ytm" => MetricId::Ytm,
            "duration_mac" => MetricId::DurationMac,
            "duration_mod" => MetricId::DurationMod,
            "convexity" => MetricId::Convexity,
            "ytw" => MetricId::Ytw,
            "dirty_price" => MetricId::DirtyPrice,
            "clean_price" => MetricId::CleanPrice,
            "cs01" => MetricId::Cs01,
            
            // IRS metrics
            "annuity" => MetricId::Annuity,
            "par_rate" => MetricId::ParRate,
            "dv01" => MetricId::Dv01,
            "pv_fixed" => MetricId::PvFixed,
            "pv_float" => MetricId::PvFloat,
            
            // Deposit metrics
            "yf" => MetricId::Yf,
            "df_start" => MetricId::DfStart,
            "df_end" => MetricId::DfEnd,
            "deposit_par_rate" => MetricId::DepositParRate,
            "df_end_from_quote" => MetricId::DfEndFromQuote,
            "quote_rate" => MetricId::QuoteRate,
            
            // Risk metrics
            "bucketed_dv01" => MetricId::BucketedDv01,
            "theta" => MetricId::Theta,
            
            // CDS metrics
            "par_spread" => MetricId::ParSpread,
            "risky_pv01" => MetricId::RiskyPv01,
            "protection_leg_pv" => MetricId::ProtectionLegPv,
            "premium_leg_pv" => MetricId::PremiumLegPv,
            
            // Option metrics
            "delta" => MetricId::Delta,
            "gamma" => MetricId::Gamma,
            "vega" => MetricId::Vega,
            "rho" => MetricId::Rho,
            
            // Any other string becomes a custom metric
            s => MetricId::Custom(s.to_string()),
        };
        Ok(metric_id)
    }
}

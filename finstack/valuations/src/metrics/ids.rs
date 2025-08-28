#![deny(missing_docs)]
//! Strongly-typed metric identifiers for compile-time validation.

use std::fmt;
use std::str::FromStr;

/// Strongly-typed metric identifier.
/// 
/// Using an enum provides compile-time validation, autocomplete support,
/// and safe refactoring when metric names change.
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
    
    // Custom metrics
    /// Custom metric with a dynamic identifier
    Custom(String),
}

impl MetricId {
    /// Create a custom metric ID.
    pub fn custom(id: impl Into<String>) -> Self {
        MetricId::Custom(id.into())
    }
    
    /// Convert to string representation for compatibility.
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
            
            // Custom metrics
            MetricId::Custom(s) => s.as_str(),
        }
    }
    

    
    /// All standard (non-custom) metric IDs.
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
    ];
}

impl fmt::Display for MetricId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MetricId {
    type Err = (); // Never fails since we have a catch-all Custom variant
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let metric_id = match s {
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
            
            // Any other string becomes a custom metric
            s => MetricId::Custom(s.to_string()),
        };
        Ok(metric_id)
    }
}

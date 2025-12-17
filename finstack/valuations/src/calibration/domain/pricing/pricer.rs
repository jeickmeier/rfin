//! Shared instrument pricing logic for curve calibration.
//!
//! This module intentionally keeps `pricer.rs` small and delegates implementation to
//! focused submodules. The public surface remains `CalibrationPricer` + `RatesQuoteUseCase`.

mod futures;
mod rates_pricing;
mod settlement;

#[cfg(test)]
mod tests;

use super::convexity::ConvexityParameters;
use finstack_core::dates::{BusinessDayConvention, Date};
use finstack_core::types::{CurveId, Currency};
use serde::{Deserialize, Serialize};

// =============================================================================
// Quote Validation Types
// =============================================================================

/// Specifies the intended use case for rate quote validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RatesQuoteUseCase {
    /// Validation for discount curve calibration.
    DiscountCurve {
        /// If true, error on forward-dependent instruments; if false, warn only.
        enforce_separation: bool,
    },
    /// Validation for forward curve calibration.
    ForwardCurve,
}

/// Instrument pricer for curve calibration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationPricer {
    /// Base date for pricing
    pub base_date: Date,
    /// Discount curve ID for pricing
    pub discount_curve_id: CurveId,
    /// Forward curve ID for floating leg projections
    pub forward_curve_id: CurveId,
    /// Settlement lag in business days (None = use quote convention or currency default)
    #[serde(default)]
    pub settlement_days: Option<i32>,
    /// Schedule/calendar identifier for settlement and date adjustments.
    ///
    /// If `None`, pricing will use the quote convention (if provided) or the
    /// market default for the calibration currency.
    #[serde(default)]
    pub calendar_id: Option<String>,
    /// Business day convention for settlement and schedule date adjustments.
    ///
    /// If `None`, pricing will use the quote convention (if provided) or the
    /// market default for the calibration currency.
    #[serde(default)]
    pub business_day_convention: Option<BusinessDayConvention>,
    /// Allow calendar-day settlement fallback
    #[serde(default)]
    pub allow_calendar_fallback: bool,
    /// Enable strict pricing (no implicit currency-based convention fallbacks).
    #[serde(default)]
    pub strict_pricing: bool,
    /// Default payment delay in business days (step-level).
    #[serde(default)]
    pub default_payment_delay_days: Option<i32>,
    /// Default reset lag in business days (step-level).
    #[serde(default)]
    pub default_reset_lag_days: Option<i32>,
    /// Use settlement date as instrument start (true for discount curves)
    #[serde(default = "default_use_settlement_start")]
    pub use_settlement_start: bool,
    /// Optional convexity parameters for futures pricing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convexity_params: Option<ConvexityParameters>,
    /// Tenor in years for forward curve (used for basis swap curve resolution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenor_years: Option<f64>,
    /// Enable verbose logging during pricing
    #[serde(default)]
    pub verbose: bool,
}

fn default_use_settlement_start() -> bool {
    true
}

impl CalibrationPricer {
    /// Market-standard calendar identifier for rates by currency.
    ///
    /// These identifiers must exist in `CalendarRegistry`.
    pub fn market_calendar_id(currency: Currency) -> &'static str {
        match currency {
            Currency::USD => "usny",
            Currency::EUR => "target2",
            Currency::GBP => "gblo",
            Currency::JPY => "jpto",
            Currency::CHF => "chzu",
            Currency::AUD => "ausy",
            Currency::CAD => "cato",
            Currency::NZD => "nzau",
            Currency::HKD => "hkex",
            Currency::SGD => "sgex",
            _ => "usny",
        }
    }

    /// Market-standard spot settlement lag (business days) by currency.
    pub fn market_settlement_days(currency: Currency) -> i32 {
        match currency {
            Currency::GBP => 0,
            Currency::AUD | Currency::CAD => 1,
            _ => 2,
        }
    }

    /// Market-standard business day convention for rates scheduling.
    pub fn market_business_day_convention(_currency: Currency) -> BusinessDayConvention {
        BusinessDayConvention::ModifiedFollowing
    }

    /// Create a new calibration pricer with defaults.
    pub fn new(base_date: Date, curve_id: impl Into<CurveId>) -> Self {
        let curve_id = curve_id.into();
        Self {
            base_date,
            discount_curve_id: curve_id.clone(),
            forward_curve_id: curve_id,
            settlement_days: None,
            calendar_id: None,
            business_day_convention: None,
            allow_calendar_fallback: false,
            strict_pricing: false,
            default_payment_delay_days: None,
            default_reset_lag_days: None,
            use_settlement_start: true,
            convexity_params: None,
            tenor_years: None,
            verbose: false,
        }
    }

    /// Create a pricer configured for forward curve calibration.
    pub fn for_forward_curve(
        base_date: Date,
        forward_curve_id: impl Into<CurveId>,
        discount_curve_id: impl Into<CurveId>,
        tenor_years: f64,
    ) -> Self {
        Self {
            base_date,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            settlement_days: None,
            calendar_id: None,
            business_day_convention: None,
            allow_calendar_fallback: false,
            strict_pricing: false,
            default_payment_delay_days: None,
            default_reset_lag_days: None,
            use_settlement_start: false,
            convexity_params: None,
            tenor_years: Some(tenor_years),
            verbose: false,
        }
    }

    /// Set the discount curve ID.
    pub fn with_discount_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.discount_curve_id = curve_id.into();
        self
    }

    /// Set the forward curve ID.
    pub fn with_forward_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.forward_curve_id = curve_id.into();
        self
    }

    /// Set explicit settlement days (overrides quote convention and currency default).
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Set the calendar identifier used for settlement/schedule generation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Set the business day convention used for settlement/schedule generation.
    pub fn with_business_day_convention(mut self, bdc: BusinessDayConvention) -> Self {
        self.business_day_convention = Some(bdc);
        self
    }

    /// Populate missing pricer-level conventions using market defaults for the given currency.
    ///
    /// Quote-level conventions still take precedence at pricing time.
    pub fn with_market_conventions(mut self, currency: Currency) -> Self {
        if self.settlement_days.is_none() {
            self.settlement_days = Some(Self::market_settlement_days(currency));
        }
        if self.calendar_id.is_none() {
            self.calendar_id = Some(Self::market_calendar_id(currency).to_string());
        }
        if self.business_day_convention.is_none() {
            self.business_day_convention = Some(Self::market_business_day_convention(currency));
        }
        self
    }

    /// Allow (or disallow) calendar-day settlement fallback.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Enable or disable strict pricing mode.
    pub fn with_strict_pricing(mut self, strict: bool) -> Self {
        self.strict_pricing = strict;
        self
    }

    /// Set the step-level default payment delay days (after period end).
    pub fn with_default_payment_delay_days(mut self, days: i32) -> Self {
        self.default_payment_delay_days = Some(days);
        self
    }

    /// Set the step-level default reset lag days (fixing offset from period start).
    pub fn with_default_reset_lag_days(mut self, days: i32) -> Self {
        self.default_reset_lag_days = Some(days);
        self
    }

    /// Set whether to use settlement date as instrument start.
    pub fn with_use_settlement_start(mut self, use_settlement: bool) -> Self {
        self.use_settlement_start = use_settlement;
        self
    }

    /// Set convexity parameters for futures pricing.
    pub fn with_convexity_params(mut self, params: ConvexityParameters) -> Self {
        self.convexity_params = Some(params);
        self
    }

    /// Set tenor in years for forward curve (used in basis swap resolution).
    pub fn with_tenor_years(mut self, tenor: f64) -> Self {
        self.tenor_years = Some(tenor);
        self
    }

    /// Enable or disable verbose logging.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Resolve forward curve ID for basis swap legs.
    pub fn resolve_forward_curve_id(&self, index_name: &str) -> CurveId {
        if let Some(tenor) = self.tenor_years {
            // Use .round() to avoid float precision issues (e.g., 0.25 * 12 = 2.9999...)
            let tenor_months = (tenor * 12.0).round() as i32;
            let token = format!("{}M", tenor_months).to_ascii_uppercase();

            // Tokenize on non-alphanumerics to avoid substring traps ("13M" contains "3M")
            let normalized = index_name.to_ascii_uppercase();
            let tokens: Vec<&str> = normalized
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .collect();

            let matches_tenor =
                tokens.contains(&token.as_str()) || (tenor_months == 12 && tokens.contains(&"1Y"));

            if matches_tenor {
                return self.forward_curve_id.clone();
            }
        }
        // Default: derive from index name
        format!("FWD_{}", index_name).into()
    }
}



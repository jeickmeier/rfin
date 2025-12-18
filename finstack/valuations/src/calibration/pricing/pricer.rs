//! Shared instrument pricing logic for curve calibration.
//!
//! This module intentionally keeps `pricer.rs` small and delegates implementation to
//! focused submodules. The public surface remains `CalibrationPricer` + `RatesQuoteUseCase`.

mod futures;
mod rates_pricing;
mod settlement;

#[cfg(test)]
mod tests;

use super::{ConvexityParameters, RatesStepConventions};
use finstack_core::dates::{BusinessDayConvention, Date};
use finstack_core::types::{Currency, CurveId};
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
///
/// This struct centralizes instrument construction and PV calculation
/// during the calibration process. It maintains base dates, curve
/// identifiers, and pricing conventions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationPricer {
    /// Base date for pricing (usually the valuation date).
    pub base_date: Date,
    /// Identifier for the discount curve used to calculate PV.
    pub discount_curve_id: CurveId,
    /// Identifier for the forward curve used for projection.
    pub forward_curve_id: CurveId,
    /// Step-level conventions for pricing and settlement.
    pub conventions: RatesStepConventions,
    /// Optional tenor in years for forward curve resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenor_years: Option<f64>,
    /// Enable verbose logging during pricing for debugging.
    #[serde(default)]
    pub verbose: bool,
}

impl CalibrationPricer {
    // =========================================================================
    // Internal helpers (perf-critical)
    // =========================================================================

    /// Return `true` if any ASCII-alphanumeric token in `s` represents the given tenor in months.
    ///
    /// This is a **zero-allocation** scanner used in hot-path quote dispatch.
    /// Recognizes tokens like `3M`, `12M`, `1Y`, `2Y` (case-insensitive).
    pub(crate) fn has_tenor_token_months(s: &str, tenor_months: i32) -> bool {
        // Fast path: impossible/invalid
        if tenor_months <= 0 {
            return false;
        }

        // Tokenize on non-alphanumeric without allocating (maximal ASCII-alnum substrings).
        let bytes = s.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            while i < bytes.len() && !bytes[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                i += 1;
            }
            if start < i {
                if let Some(m) = Self::parse_tenor_token_months(&s[start..i]) {
                    if m == tenor_months {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Parse a token like `3M` or `1Y` (case-insensitive) into months.
    #[inline]
    fn parse_tenor_token_months(tok: &str) -> Option<i32> {
        let b = tok.as_bytes();
        if b.len() < 2 {
            return None;
        }
        let last = b[b.len() - 1];
        let unit = last.to_ascii_uppercase();
        if unit != b'M' && unit != b'Y' {
            return None;
        }

        // Parse integer prefix (no allocation).
        let mut n: i32 = 0;
        for &ch in &b[..b.len() - 1] {
            if !ch.is_ascii_digit() {
                return None;
            }
            n = n.saturating_mul(10).saturating_add((ch - b'0') as i32);
        }
        if n <= 0 {
            return None;
        }
        Some(if unit == b'Y' {
            n.saturating_mul(12)
        } else {
            n
        })
    }

    /// Market-standard calendar identifier for rates by currency.
    ///
    /// These identifiers must exist in `CalendarRegistry`.
    pub(crate) fn market_calendar_id(currency: Currency) -> &'static str {
        crate::calibration::quotes::conventions::DepositConventions::for_currency(currency)
            .calendar_id
            .as_str()
    }

    /// Market-standard spot settlement lag (business days) by currency.
    pub(crate) fn market_settlement_days(currency: Currency) -> i32 {
        crate::calibration::quotes::conventions::DepositConventions::for_currency(currency)
            .settlement_days
    }

    /// Market-standard business day convention for rates scheduling.
    pub(crate) fn market_business_day_convention(_currency: Currency) -> BusinessDayConvention {
        crate::calibration::quotes::conventions::DepositConventions::for_currency(_currency)
            .business_day_convention
    }

    /// Create a new calibration pricer with defaults.
    pub fn new(base_date: Date, curve_id: impl Into<CurveId>) -> Self {
        let curve_id = curve_id.into();
        Self {
            base_date,
            discount_curve_id: curve_id.clone(),
            forward_curve_id: curve_id,
            conventions: RatesStepConventions {
                use_settlement_start: Some(true),
                ..Default::default()
            },
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
            conventions: RatesStepConventions {
                use_settlement_start: Some(false),
                ..Default::default()
            },
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
        self.conventions.settlement_days = Some(days);
        self
    }

    /// Set the calendar identifier used for settlement/schedule generation.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.conventions.calendar_id = Some(calendar_id.into());
        self
    }

    /// Set the business day convention used for settlement/schedule generation.
    pub fn with_business_day_convention(mut self, bdc: BusinessDayConvention) -> Self {
        self.conventions.business_day_convention = Some(bdc);
        self
    }

    /// Populate missing conventions using market defaults for a currency.
    pub fn with_market_conventions(mut self, currency: Currency) -> Self {
        if self.conventions.settlement_days.is_none() {
            self.conventions.settlement_days = Some(Self::market_settlement_days(currency));
        }
        if self.conventions.calendar_id.is_none() {
            self.conventions.calendar_id = Some(Self::market_calendar_id(currency).to_string());
        }
        if self.conventions.business_day_convention.is_none() {
            self.conventions.business_day_convention =
                Some(Self::market_business_day_convention(currency));
        }
        self
    }

    /// Allow (or disallow) calendar-day settlement fallback.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.conventions.allow_calendar_fallback = Some(allow);
        self
    }

    /// Enable or disable strict pricing mode.
    pub fn with_strict_pricing(mut self, strict: bool) -> Self {
        self.conventions.strict_pricing = Some(strict);
        self
    }

    /// Set the step-level default payment delay days (after period end).
    pub fn with_default_payment_delay_days(mut self, days: i32) -> Self {
        self.conventions.default_payment_delay_days = Some(days);
        self
    }

    /// Set the step-level default reset lag days (fixing offset from period start).
    pub fn with_default_reset_lag_days(mut self, days: i32) -> Self {
        self.conventions.default_reset_lag_days = Some(days);
        self
    }

    /// Set whether to use settlement date as instrument start.
    pub fn with_use_settlement_start(mut self, use_settlement: bool) -> Self {
        self.conventions.use_settlement_start = Some(use_settlement);
        self
    }

    /// Set convexity parameters for futures pricing.
    pub fn with_convexity_params(mut self, params: ConvexityParameters) -> Self {
        self.conventions.convexity_params = Some(params);
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

    /// Resolve the forward curve identifier for a basis swap leg.
    pub fn resolve_forward_curve_id(&self, index_name: &str) -> CurveId {
        if let Some(tenor) = self.tenor_years {
            // Use .round() to avoid float precision issues (e.g., 0.25 * 12 = 2.9999...)
            let tenor_months = (tenor * 12.0).round() as i32;
            // PERF: zero-allocation token scan (avoid uppercasing + Vec collection).
            let matches_tenor = Self::has_tenor_token_months(index_name, tenor_months);

            if matches_tenor {
                return self.forward_curve_id.clone();
            }
        }
        // Default: derive from index name
        format!("FWD_{}", index_name).into()
    }
}

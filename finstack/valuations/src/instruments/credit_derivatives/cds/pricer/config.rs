use crate::constants::time as time_constants;

/// Configuration for CDS pricing.
///
/// Controls numerical integration and par spread calculation methodology.
/// Use factory methods like [`isda_standard()`](Self::isda_standard) for
/// pre-configured setups.
#[derive(Debug, Clone)]
pub(crate) struct CDSPricerConfig {
    /// Include accrual on default in premium leg calculation
    pub(crate) include_accrual: bool,
    /// Par spread denominator methodology:
    /// - `false` (default): Use Risky Annuity only (ISDA Standard Model)
    /// - `true`: Include accrual-on-default in denominator (Bloomberg CDSW style)
    ///
    /// The difference is typically < 1bp for investment grade but can reach 2-5 bps
    /// for distressed credits (hazard rate > 3%).
    pub(crate) par_spread_uses_full_premium: bool,
    /// Business days per year for settlement delay calculations (region-specific).
    /// Default: 252 (US), alternatives: 250 (UK), 255 (Japan).
    /// Only consulted when no calendar is attached to the CDS premium leg.
    pub(crate) business_days_per_year: f64,
}

impl Default for CDSPricerConfig {
    fn default() -> Self {
        Self::isda_standard()
    }
}

impl CDSPricerConfig {
    /// Create an ISDA 2014 standard compliant configuration (North America/US market).
    ///
    /// Features:
    /// - ISDA Standard Model integration (analytical piecewise-constant)
    /// - ISDA coupon dates (20th of Mar/Jun/Sep/Dec)
    /// - Accrual-on-default included
    /// - Risky annuity for par spread denominator
    #[must_use]
    pub(crate) fn isda_standard() -> Self {
        Self {
            include_accrual: true,
            par_spread_uses_full_premium: false,
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_US,
        }
    }

    /// Build a CDS pricer configuration from instrument-level valuation policy.
    #[must_use]
    pub(crate) fn from_cds(
        cds: &crate::instruments::credit_derivatives::cds::CreditDefaultSwap,
    ) -> Self {
        Self {
            par_spread_uses_full_premium: cds.uses_full_premium_par_spread_denominator(),
            ..Self::isda_standard()
        }
    }
}


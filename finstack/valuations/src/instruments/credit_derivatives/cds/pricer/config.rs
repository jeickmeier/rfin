use super::IntegrationMethod;
use crate::constants::{isda, numerical, time as time_constants};
use crate::instruments::credit_derivatives::cds::CdsDocClause;
use finstack_core::{Error, Result};

/// Configuration for CDS pricing.
///
/// Controls numerical integration, day count conventions, and par spread calculation
/// methodology. Use factory methods like [`isda_standard()`](Self::isda_standard) for
/// pre-configured setups.
#[derive(Debug, Clone)]
pub(crate) struct CDSPricerConfig {
    /// Number of integration steps per year for protection leg (used with Midpoint method).
    pub(crate) steps_per_year: usize,
    /// Minimum integration steps per year (floor for adaptive step calculation).
    pub(crate) min_steps_per_year: usize,
    /// If true, adapt integration steps based on tenor: `max(min_steps_per_year, tenor * 12)`.
    /// Provides higher accuracy for longer tenors and distressed credits.
    pub(crate) adaptive_steps: bool,
    /// Include accrual on default in premium leg calculation
    pub(crate) include_accrual: bool,
    /// Integration method for protection leg calculation
    pub(crate) integration_method: IntegrationMethod,
    /// Use ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec)
    pub(crate) use_isda_coupon_dates: bool,
    /// Par spread denominator methodology:
    /// - `false` (default): Use Risky Annuity only (ISDA Standard Model)
    /// - `true`: Include accrual-on-default in denominator (Bloomberg CDSW style)
    ///
    /// The difference is typically < 1bp for investment grade but can reach 2-5 bps
    /// for distressed credits (hazard rate > 3%).
    pub(crate) par_spread_uses_full_premium: bool,
    /// If true, apply the current restructuring-clause approximation to the protection leg.
    ///
    /// Default is `false` because the approximation is not clause-consistent enough for
    /// production pricing. When enabled, protection PV ordering follows
    /// `Xr14 <= Mr14 <= Mm14 <= Cr14` heuristically.
    pub(crate) enable_restructuring_approximation: bool,
    /// Business days per year for settlement delay calculations (region-specific).
    /// Default: 252 (US), alternatives: 250 (UK), 255 (Japan)
    pub(crate) business_days_per_year: f64,
    /// Max iterations for bootstrapping solver
    pub(crate) bootstrap_max_iterations: usize,
    /// Tolerance for bootstrapping solver
    pub(crate) bootstrap_tolerance: f64,
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
    /// - Adaptive step sizing based on tenor
    /// - ISDA coupon dates (20th of Mar/Jun/Sep/Dec)
    /// - Accrual-on-default included
    /// - Risky annuity for par spread denominator
    #[must_use]
    pub(crate) fn isda_standard() -> Self {
        Self {
            steps_per_year: isda::STANDARD_INTEGRATION_POINTS,
            min_steps_per_year: isda::STANDARD_INTEGRATION_POINTS,
            adaptive_steps: true,
            include_accrual: true,
            integration_method: IntegrationMethod::IsdaStandardModel,
            use_isda_coupon_dates: true,
            par_spread_uses_full_premium: false,
            enable_restructuring_approximation: false,
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_US,
            bootstrap_max_iterations: 100,
            bootstrap_tolerance: numerical::SOLVER_TOLERANCE,
        }
    }

    /// Create an ISDA configuration for European markets (UK conventions).
    #[must_use]
    pub(crate) fn isda_europe() -> Self {
        Self {
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_UK,
            ..Self::isda_standard()
        }
    }

    /// Create an ISDA configuration for Asian markets (Japan conventions).
    #[must_use]
    pub(crate) fn isda_asia() -> Self {
        Self {
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_JP,
            ..Self::isda_standard()
        }
    }

    /// Create a simplified configuration for faster but less accurate pricing.
    ///
    /// Uses midpoint integration without adaptive steps. Suitable for
    /// approximate valuations or high-volume batch processing.
    #[must_use]
    pub(crate) fn simplified() -> Self {
        Self {
            steps_per_year: 365,
            min_steps_per_year: 52,
            adaptive_steps: false,
            include_accrual: true,
            integration_method: IntegrationMethod::Midpoint,
            use_isda_coupon_dates: false,
            par_spread_uses_full_premium: false,
            enable_restructuring_approximation: false,
            business_days_per_year: time_constants::BUSINESS_DAYS_PER_YEAR_US,
            bootstrap_max_iterations: 100,
            bootstrap_tolerance: numerical::SOLVER_TOLERANCE,
        }
    }

    /// Calculate effective integration steps based on tenor.
    ///
    /// When `adaptive_steps` is enabled, returns `max(min_steps_per_year, tenor_years * 12)`.
    /// This ensures higher accuracy for longer tenors and distressed credits.
    #[must_use]
    pub(crate) fn effective_steps(&self, tenor_years: f64) -> usize {
        if self.adaptive_steps {
            let adaptive = (tenor_years * 12.0).ceil() as usize;
            self.min_steps_per_year.max(adaptive)
        } else {
            self.steps_per_year
        }
    }

    /// Validate configuration parameters.
    ///
    /// Returns an error if any parameter is out of valid range. This method provides
    /// fail-fast validation for catching configuration errors early.
    ///
    /// # Errors
    ///
    /// Returns a validation error if:
    /// - `steps_per_year` is zero
    /// - `min_steps_per_year` is zero
    /// - `bootstrap_max_iterations` is zero
    /// - `bootstrap_tolerance` is not positive
    /// - `business_days_per_year` is not positive
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::credit_derivatives::cds::CDSPricerConfig;
    ///
    /// let config = CDSPricerConfig::isda_standard();
    /// assert!(config.validate().is_ok());
    /// ```
    pub(crate) fn validate(&self) -> Result<()> {
        if self.steps_per_year == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: steps_per_year must be at least 1".into(),
            ));
        }
        if self.min_steps_per_year == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: min_steps_per_year must be at least 1".into(),
            ));
        }
        if self.bootstrap_max_iterations == 0 {
            return Err(Error::Validation(
                "CDSPricerConfig: bootstrap_max_iterations must be at least 1".into(),
            ));
        }
        if self.bootstrap_tolerance <= 0.0 {
            return Err(Error::Validation(
                "CDSPricerConfig: bootstrap_tolerance must be positive".into(),
            ));
        }
        if self.business_days_per_year <= 0.0 {
            return Err(Error::Validation(
                "CDSPricerConfig: business_days_per_year must be positive".into(),
            ));
        }
        Ok(())
    }
}

/// Maximum deliverable obligation maturity cap (in months) for a given
/// documentation clause.
///
/// This controls how restructuring credit events affect the protection leg:
///
/// - **`Cr14`** (Full Restructuring): No maturity cap on deliverable obligations.
///   All bonds of the reference entity are deliverable, making restructuring a
///   broad credit event. Returns `None` (uncapped).
///
/// - **`Mr14`** (Modified Restructuring): Deliverable obligations are capped at
///   30 months from the restructuring event. This limits the cheapest-to-deliver
///   option and reduces the value of restructuring protection.
///
/// - **`Mm14`** (Modified-Modified Restructuring): 60-month cap on deliverable
///   obligation maturity. A compromise between CR and MR, common in European CDS.
///
/// - **`Xr14`** (No Restructuring): Restructuring is not a credit event.
///   Returns `Some(0)` indicating no restructuring benefit.
///
/// - **Meta-clauses** (`IsdaNa`, `IsdaEu`, `IsdaAs`, `IsdaAu`, `IsdaNz`):
///   Delegate to the effective concrete clause per regional convention.
///
/// - **`Custom`**: Treated as no restructuring (`Some(0)`) by default.
///
/// # Returns
///
/// - `None`: No maturity cap (full restructuring benefit).
/// - `Some(0)`: No restructuring benefit (Xr14 or Custom).
/// - `Some(n)`: Maturity cap of `n` months from the restructuring event.
#[must_use]
pub(crate) fn max_deliverable_maturity(clause: CdsDocClause) -> Option<u32> {
    match clause {
        CdsDocClause::Cr14 => None,      // Full restructuring, uncapped
        CdsDocClause::Mr14 => Some(30),  // Modified Restructuring: 30 months
        CdsDocClause::Mm14 => Some(60),  // Modified-Modified Restructuring: 60 months
        CdsDocClause::Xr14 => Some(0),   // No Restructuring: no benefit
        CdsDocClause::Custom => Some(0), // Conservative default: no benefit
        // Meta-clauses delegate to their effective concrete clause
        CdsDocClause::IsdaNa => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaEu => max_deliverable_maturity(CdsDocClause::Mm14),
        CdsDocClause::IsdaAs => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaAu => max_deliverable_maturity(CdsDocClause::Xr14),
        CdsDocClause::IsdaNz => max_deliverable_maturity(CdsDocClause::Xr14),
    }
}

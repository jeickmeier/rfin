use crate::constants::time as time_constants;
use crate::instruments::credit_derivatives::cds::CdsDocClause;

/// Configuration for CDS pricing.
///
/// Controls numerical integration, day count conventions, and par spread calculation
/// methodology. Use factory methods like [`isda_standard()`](Self::isda_standard) for
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
    /// If true, apply the current restructuring-clause approximation to the protection leg.
    ///
    /// Default is `false` because the approximation is not clause-consistent enough for
    /// production pricing. When enabled, protection PV ordering follows
    /// `Xr14 <= Mr14 <= Mm14 <= Cr14` heuristically.
    pub(crate) enable_restructuring_approximation: bool,
    /// Business days per year for settlement delay calculations (region-specific).
    /// Default: 252 (US), alternatives: 250 (UK), 255 (Japan)
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
            enable_restructuring_approximation: false,
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

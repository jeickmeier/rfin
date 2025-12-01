//! Hazard curve bootstrapping from CDS par spreads.
//!
//! Calibrates a piecewise-constant hazard curve by matching CDS par spreads
//! sequentially across maturities using an objective that drives the CDS NPV
//! to ~0 at the quoted spread.
//!
//! # Market Standards
//!
//! This implementation follows the **ISDA CDS Standard Model (2009)** for:
//! - Day count conventions (ACT/360 for NA/EU, ACT/365F for Asia)
//! - Quarterly premium payment schedules (20th of Mar/Jun/Sep/Dec)
//! - Recovery rate assumptions (40% for senior unsecured, 25% for subordinated)
//! - Settlement timing (T+3 for upfront payments per Big Bang protocol)
//!
//! # Recovery Rate Handling
//!
//! The calibrator enforces recovery rate consistency between quotes and the
//! calibrator configuration. All quotes for a given entity must use the same
//! recovery rate assumption as specified in the calibrator constructor.
//!
//! # References
//!
//! - ISDA (2009). "ISDA CDS Standard Model." Version 1.8.2.
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*. Wiley.
//! - Markit (2009). "CDS Curve Bootstrapping Guide."

use crate::calibration::quote::CreditQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::constants::time as time_constants;
use crate::instruments::cds::pricer::CDSPricer;
use crate::instruments::cds::{CDSConvention, CreditDefaultSwap, PayReceive};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{HazardCurve, ParInterp, Seniority};
use finstack_core::market_data::traits::Discounting;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use smallvec::SmallVec;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Type alias for hazard curve knots stored on stack (up to 16 tenors).
/// This avoids heap allocations for typical CDS curves (1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 20Y, 30Y).
type HazardKnots = SmallVec<[(f64, f64); 16]>;

/// Recovery rate tolerance for consistency validation (0.01 = 1 percentage point).
const RECOVERY_TOLERANCE: f64 = 0.01;

/// Hazard curve bootstrapper using CDS par spreads.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::HazardCurveCalibrator;
/// use finstack_valuations::calibration::{Calibrator, CreditQuote};
///
/// let calibrator = HazardCurveCalibrator::new(
///     "CORP",
///     Seniority::Senior,
///     0.40,  // Recovery rate must match quotes
///     base_date,
///     Currency::USD,
///     "USD-OIS",
/// );
///
/// let (hazard_curve, report) = calibrator.calibrate(&quotes, &market_context)?;
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HazardCurveCalibrator {
    /// Reference entity name
    pub entity: String,
    /// Seniority level (metadata)
    pub seniority: Seniority,
    /// Recovery rate assumption (must match all quote recovery rates)
    pub recovery_rate: f64,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Currency (metadata)
    pub currency: Currency,
    /// Discount curve identifier for collateral discounting
    pub discount_curve_id: CurveId,
    /// CDS convention for day count and schedule generation
    pub convention: CDSConvention,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Interpolation used when reporting quoted par spreads from the calibrated curve
    pub par_interp: ParInterp,
}

impl HazardCurveCalibrator {
    /// Helper to determine default discount curve ID from currency.
    /// Uses common market conventions for collateral.
    #[must_use]
    pub fn default_discount_curve_id(currency: Currency) -> CurveId {
        CurveId::new(format!("{}-OIS", currency))
    }

    /// Determine default CDS convention from currency.
    ///
    /// - USD/CAD: ISDA North America (ACT/360)
    /// - EUR/GBP/CHF: ISDA Europe (ACT/360)
    /// - JPY/HKD/SGD/AUD/NZD: ISDA Asia (ACT/365F)
    #[must_use]
    pub fn default_convention(currency: Currency) -> CDSConvention {
        match currency {
            Currency::USD | Currency::CAD => CDSConvention::IsdaNa,
            Currency::EUR | Currency::GBP | Currency::CHF => CDSConvention::IsdaEu,
            Currency::JPY | Currency::HKD | Currency::SGD | Currency::AUD | Currency::NZD => {
                CDSConvention::IsdaAs
            }
            _ => CDSConvention::IsdaNa, // Default to NA for other currencies
        }
    }

    /// Create a new hazard curve calibrator.
    ///
    /// # Arguments
    ///
    /// * `entity` - Reference entity name (must match `entity` field in quotes)
    /// * `seniority` - Debt seniority level for the curve
    /// * `recovery_rate` - Recovery rate assumption (must match all quote recovery rates)
    /// * `base_date` - Valuation date for the curve
    /// * `currency` - Currency of protection leg
    /// * `discount_curve_id` - Identifier for the discount curve used in PV calculations
    ///
    /// # Note
    ///
    /// Uses default CDS convention based on currency. For explicit convention control,
    /// use `with_convention()` builder method.
    pub fn new(
        entity: impl Into<String>,
        seniority: Seniority,
        recovery_rate: f64,
        base_date: finstack_core::dates::Date,
        currency: Currency,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&recovery_rate),
            "Recovery rate must be in [0, 1]"
        );
        Self {
            entity: entity.into(),
            seniority,
            recovery_rate,
            base_date,
            currency,
            discount_curve_id: discount_curve_id.into(),
            convention: Self::default_convention(currency),
            config: CalibrationConfig::default(),
            par_interp: ParInterp::Linear,
        }
    }

    /// Create a new hazard curve calibrator using default discount curve ID.
    /// This is a convenience method that uses standard OIS curves based on currency.
    pub fn new_with_default_discount(
        entity: impl Into<String>,
        seniority: Seniority,
        recovery_rate: f64,
        base_date: finstack_core::dates::Date,
        currency: Currency,
    ) -> Self {
        let discount_curve_id = Self::default_discount_curve_id(currency);
        Self::new(
            entity,
            seniority,
            recovery_rate,
            base_date,
            currency,
            discount_curve_id,
        )
    }

    /// Set the CDS convention for day count and schedule generation.
    ///
    /// By default, the convention is inferred from currency. Use this method
    /// to override with a specific convention.
    #[must_use]
    pub fn with_convention(mut self, convention: CDSConvention) -> Self {
        self.convention = convention;
        self
    }

    /// Set calibration configuration.
    #[must_use]
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the interpolation used for reporting par spreads from the hazard curve.
    #[must_use]
    pub fn with_par_interp(mut self, method: ParInterp) -> Self {
        self.par_interp = method;
        self
    }

    /// Generate adaptive grid for root bracketing based on initial guess.
    ///
    /// Creates a log-spaced grid spanning approximately 4 orders of magnitude
    /// around the initial guess. This handles both investment-grade (low spreads)
    /// and distressed (high spreads) credits effectively.
    fn adaptive_grid(initial_guess: f64) -> [f64; 16] {
        let center = initial_guess.clamp(1e-6, 2.0);
        let log_center = center.log10();

        // Generate grid spanning from ~1e-6 to ~2.0 (200% hazard rate)
        // with more density around the expected region
        let low_exp = (log_center - 2.0).max(-6.0);
        let high_exp = (log_center + 1.5).min(0.3); // Max ~2.0

        let step = (high_exp - low_exp) / 15.0;

        let mut grid = [0.0; 16];
        for (i, g) in grid.iter_mut().enumerate() {
            *g = 10f64.powf(low_exp + step * i as f64);
        }
        grid
    }

    /// Validate recovery rate consistency between quotes and calibrator.
    fn validate_recovery_consistency(&self, quotes: &[CreditQuote]) -> Result<()> {
        for q in quotes {
            let (entity, quote_recovery) = match q {
                CreditQuote::CDS {
                    entity,
                    recovery_rate,
                    ..
                } => (entity, *recovery_rate),
                CreditQuote::CDSUpfront {
                    entity,
                    recovery_rate,
                    ..
                } => (entity, *recovery_rate),
                _ => continue,
            };

            // Only validate quotes for this entity
            if entity != &self.entity {
                continue;
            }

            if (quote_recovery - self.recovery_rate).abs() > RECOVERY_TOLERANCE {
                return Err(finstack_core::Error::Validation(format!(
                    "Recovery rate mismatch for entity '{}': quote has {:.1}% but calibrator expects {:.1}%. \
                     All quotes must use the same recovery rate as specified in the calibrator.",
                    entity,
                    quote_recovery * 100.0,
                    self.recovery_rate * 100.0
                )));
            }
        }
        Ok(())
    }

    fn bootstrap_internal<S: finstack_core::math::Solver>(
        &self,
        quotes: &[CreditQuote],
        solver: &S,
        discount_curve_opt: Option<&dyn Discounting>,
    ) -> Result<(HazardCurve, CalibrationReport)> {
        // Validate recovery rate consistency upfront
        self.validate_recovery_consistency(quotes)?;

        // Get day count from convention
        let day_count = self.convention.day_count();

        // Extract CDS quotes for this entity and sort by maturity
        let mut cds_quotes: Vec<(finstack_core::dates::Date, f64, Option<f64>)> = quotes
            .iter()
            .filter_map(|q| match q {
                CreditQuote::CDS {
                    entity,
                    maturity,
                    spread_bp,
                    ..
                } if entity == &self.entity => Some((*maturity, *spread_bp, None)),
                CreditQuote::CDSUpfront {
                    entity,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                    ..
                } if entity == &self.entity => {
                    Some((*maturity, *running_spread_bp, Some(*upfront_pct)))
                }
                _ => None,
            })
            .collect();

        if cds_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Validate all spreads are positive (zero or negative spreads are invalid)
        for (maturity, spread_bp, _) in &cds_quotes {
            if *spread_bp <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "CDS spread must be positive, got {} bp for maturity {}. \
                     Zero or negative spreads imply no credit risk and cannot be calibrated.",
                    spread_bp, maturity
                )));
            }
        }

        // Sort by maturity using total_cmp for safe float comparison
        cds_quotes.sort_by(|a, b| a.0.cmp(&b.0));

        // Sequentially solve hazards per tenor to match market PV≈0
        // Use SmallVec to avoid heap allocations for typical curve sizes
        let mut hazard_knots: HazardKnots = SmallVec::new();
        let mut par_knots: HazardKnots = SmallVec::new();
        let mut residuals: BTreeMap<String, f64> = BTreeMap::new();
        let mut total_iterations: usize = 0;

        // Get settlement discount factor for upfront quotes (T+3 settlement)
        let settlement_delay_days = self.convention.settlement_delay() as f64;
        let business_days_per_year = match self.convention {
            CDSConvention::IsdaNa => time_constants::BUSINESS_DAYS_PER_YEAR_US,
            CDSConvention::IsdaEu => time_constants::BUSINESS_DAYS_PER_YEAR_UK,
            CDSConvention::IsdaAs => time_constants::BUSINESS_DAYS_PER_YEAR_JP,
            CDSConvention::Custom => time_constants::BUSINESS_DAYS_PER_YEAR_US,
        };
        let settlement_delay_years = settlement_delay_days / business_days_per_year;

        for (maturity, market_spread_bp, upfront_pct_opt) in &cds_quotes {
            // Time axis using convention day count
            let tenor_years = day_count
                .year_fraction(
                    self.base_date,
                    *maturity,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .map_err(|_| {
                    finstack_core::Error::Input(finstack_core::error::InputError::Invalid)
                })?;

            if tenor_years <= 0.0 {
                continue;
            }

            // Synthetic CDS at market spread
            const CALIB_HAZARD_ID: &str = "CALIB_HAZARD";
            const CALIB_DISC_ID: &str = "CALIB_DISC";

            let cds = CreditDefaultSwap::new_isda(
                format!("CALIB_CDS_{}", maturity),
                Money::new(10_000_000.0, self.currency),
                PayReceive::PayFixed,
                self.convention,
                *market_spread_bp,
                self.base_date,
                *maturity,
                self.recovery_rate,
                finstack_core::types::CurveId::new(CALIB_DISC_ID),
                finstack_core::types::CurveId::new(CALIB_HAZARD_ID),
            );

            let pricer = CDSPricer::new();

            // Copy current knots for use in closure (avoids clone on each iteration)
            let knots_snapshot: HazardKnots = hazard_knots.clone();
            let convention = self.convention;
            let recovery_rate = self.recovery_rate;
            let base_date = self.base_date;

            let objective = |trial_lambda: f64| -> f64 {
                // Build temporary hazard curve with prior segments + trial point
                // Use SmallVec to avoid heap allocation
                let mut temp_knots: HazardKnots = knots_snapshot.clone();
                temp_knots.push((tenor_years, trial_lambda.max(0.0)));

                let temp_curve = HazardCurve::builder("TEMP_CALIB")
                    .base_date(base_date)
                    .day_count(convention.day_count())
                    .recovery_rate(recovery_rate)
                    .knots(temp_knots.into_vec())
                    .build();

                let temp_curve = match temp_curve {
                    Ok(c) => c,
                    Err(_) => return crate::calibration::PENALTY,
                };
                let disc = match discount_curve_opt {
                    Some(d) => d,
                    None => return crate::calibration::PENALTY,
                };

                // Calculate CDS NPV
                let npv_result = pricer.npv(&cds, disc, &temp_curve, base_date);
                let npv = match npv_result {
                    Ok(pv) => pv.amount(),
                    Err(_) => return crate::calibration::PENALTY,
                };

                // Objective depends on quote type
                match upfront_pct_opt {
                    None => {
                        // Par spread quote: PV per $ notional ≈ 0 using quoted spread
                        npv / cds.notional.amount()
                    }
                    Some(upfront_pct) => {
                        // Upfront quote: Apply T+3 settlement discounting per ISDA Big Bang
                        let settlement_df = disc.df(settlement_delay_years);
                        let expected_upfront =
                            cds.notional.amount() * upfront_pct / 100.0 * settlement_df;
                        (npv - expected_upfront) / cds.notional.amount()
                    }
                }
            };

            // Initial guess: last solved λ or s/(1-R) approximation
            let initial_guess = hazard_knots
                .last()
                .map(|&(_, l)| l)
                .unwrap_or(*market_spread_bp / 10000.0 / (1.0 - self.recovery_rate));

            // Use adaptive grid based on initial guess for robust bracketing
            let grid = Self::adaptive_grid(initial_guess);
            let mut solved = initial_guess.max(1e-6);
            let mut bracket_found = false;
            let mut last: Option<(f64, f64)> = None;

            for &x in &grid {
                let v = objective(x);
                if let Some((px, pv)) = last {
                    if pv.is_finite() && v.is_finite() && pv.signum() != v.signum() {
                        // Found bracket - use configured 1D solver
                        let guess = 0.5 * (px + x);
                        solved = crate::calibration::solve_1d(
                            self.config.solver_kind.clone(),
                            self.config.tolerance,
                            self.config.max_iterations,
                            objective,
                            guess,
                        )?;
                        bracket_found = true;
                        break;
                    }
                }
                last = Some((x, v));
            }

            // If bracket search didn't resolve, try bounded fallback with narrow grid
            if !bracket_found || !solved.is_finite() || solved <= 0.0 {
                // Narrow grid around initial guess as fallback
                let narrow_grid = [
                    initial_guess * 0.01,
                    initial_guess * 0.1,
                    initial_guess * 0.5,
                    initial_guess,
                    initial_guess * 2.0,
                    initial_guess * 5.0,
                    initial_guess * 10.0,
                    initial_guess * 100.0,
                ];

                let mut narrow_found = false;
                let mut last_narrow: Option<(f64, f64)> = None;

                for &x in &narrow_grid {
                    let v = objective(x);
                    if let Some((px, pv)) = last_narrow {
                        if pv.is_finite() && v.is_finite() && pv.signum() != v.signum() {
                            let guess = 0.5 * (px + x);
                            solved = crate::calibration::solve_1d(
                                self.config.solver_kind.clone(),
                                self.config.tolerance,
                                self.config.max_iterations,
                                objective,
                                guess,
                            )?;
                            narrow_found = true;
                            break;
                        }
                    }
                    last_narrow = Some((x, v));
                }

                // Final fallback to unbounded solver if narrow grid also fails
                if !narrow_found || !solved.is_finite() || solved <= 0.0 {
                    solved = solver.solve(objective, initial_guess.max(1e-6))?;
                }
            }

            // Validate hazard rate is positive (market standards requirement)
            if solved <= 0.0 || !solved.is_finite() {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Calibrated hazard rate {:.6} for maturity {} is not positive. \
                         This indicates either invalid market data or calibration failure. \
                         Spread: {:.1}bp, Recovery: {:.1}%",
                        solved,
                        maturity,
                        market_spread_bp,
                        self.recovery_rate * 100.0
                    ),
                    category: "hazard_curve_negative_rate".to_string(),
                });
            }

            hazard_knots.push((tenor_years, solved));
            par_knots.push((tenor_years, *market_spread_bp));

            let res = objective(solved).abs();
            let key = match upfront_pct_opt {
                None => format!("CDS-PAR-{}", maturity),
                Some(_) => format!("CDS-UPFRONT-{}", maturity),
            };
            residuals.insert(key, res);
            total_iterations += 1;
        }

        // Build final hazard curve with stable id
        let id_owned = format!("{}-{}", self.entity, self.seniority);

        let curve = HazardCurve::builder(id_owned)
            .issuer(&self.entity)
            .seniority(self.seniority)
            .currency(self.currency)
            .recovery_rate(self.recovery_rate)
            .day_count(day_count)
            .base_date(self.base_date)
            .knots(hazard_knots.into_vec())
            .par_spreads(par_knots.into_vec())
            .par_interp(self.par_interp)
            .build()?;

        // Validate the calibrated hazard curve
        use crate::calibration::validation::{CurveValidator, ValidationConfig};
        curve.validate(&ValidationConfig::default()).map_err(|e| {
            finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated hazard curve for {} failed validation: {}",
                    self.entity, e
                ),
                category: "hazard_curve_validation".to_string(),
            }
        })?;

        let report = CalibrationReport::for_type_with_tolerance(
            "hazard_curve",
            residuals,
            total_iterations,
            self.config.tolerance,
        )
        .with_metadata("entity", self.entity.clone())
        .with_metadata("recovery_rate", format!("{:.3}", self.recovery_rate))
        .with_metadata("convention", format!("{:?}", self.convention))
        .with_metadata("par_interp", format!("{:?}", self.par_interp))
        .with_metadata("validation", "passed");

        Ok((curve, report))
    }
}

impl Calibrator<CreditQuote, HazardCurve> for HazardCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[CreditQuote],
        base_context: &MarketContext,
    ) -> Result<(HazardCurve, CalibrationReport)> {
        let disc = base_context.get_discount_ref(&self.discount_curve_id)?;
        let solver = crate::calibration::create_simple_solver(&self.config);
        self.bootstrap_internal(instruments, &solver, Some(disc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
    use time::Month;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"))
            .knots([
                (0.0, 1.0),
                (1.0, 0.95),
                (3.0, 0.90),
                (5.0, 0.85),
                (10.0, 0.75),
            ])
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
    }

    fn test_cds_quotes() -> Vec<CreditQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        vec![
            CreditQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365),
                spread_bp: 50.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 3),
                spread_bp: 75.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 100.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
        ]
    }

    #[test]
    fn hazard_calibration_reprices_cds_within_one_dollar_per_million() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let quotes = test_cds_quotes();
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "AAPL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );
        let market_context = MarketContext::new().insert_discount(disc);
        let (hazard, report) = calibrator
            .calibrate(&quotes, &market_context)
            .expect("hazard curve calibration failed");
        assert!(report.success);

        // Get the discount curve from the market context
        let disc = market_context
            .get_discount_ref("USD-OIS")
            .expect("discount curve not found");

        // Reprice each quoted CDS and assert PV per $1MM is within $1
        let pricer = CDSPricer::new();
        for q in quotes {
            if let CreditQuote::CDS {
                maturity,
                spread_bp,
                ..
            } = q
            {
                let cds = CreditDefaultSwap::new_isda(
                    format!("CDS-{}", maturity),
                    Money::new(1_000_000.0, Currency::USD),
                    PayReceive::PayFixed,
                    CDSConvention::IsdaNa,
                    spread_bp,
                    base_date,
                    maturity,
                    0.40,
                    finstack_core::types::CurveId::new("USD-OIS"),
                    finstack_core::types::CurveId::new("AAPL-Senior"),
                );

                let pv = pricer
                    .npv(&cds, disc, &hazard, base_date)
                    .expect("cds npv failed");
                assert!(
                    pv.amount().abs() <= 1.0,
                    "repricing error too large: {}",
                    pv.amount()
                );
            }
        }
    }

    #[test]
    fn hazard_calibration_basic_properties_and_metadata() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let quotes = test_cds_quotes();
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "AAPL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );
        let market_context = MarketContext::new().insert_discount(disc);
        let (hazard, report) = calibrator
            .calibrate(&quotes, &market_context)
            .expect("hazard curve calibration failed");
        assert!(report.success);

        // Day count and recovery metadata
        assert_eq!(hazard.day_count(), CDSConvention::IsdaNa.day_count());
        assert!((hazard.recovery_rate() - 0.40).abs() < 1e-12);
        assert_eq!(hazard.base_date(), base_date);
        assert_eq!(hazard.seniority, Some(Seniority::Senior));

        // Tenors strictly increasing and lambdas non-negative (check survival monotonicity)
        let times = [1.0, 3.0, 5.0];
        let mut prev_sp = 1.0;
        for &t in &times {
            let sp = hazard.sp(t);
            assert!(sp <= prev_sp + 1e-12);
            prev_sp = sp;
        }

        // Par spread retrieval at pillar times (use same day-count mapping as bootstrap)
        let dc = hazard.day_count();
        let t1 = dc
            .year_fraction(
                base_date,
                base_date + time::Duration::days(365),
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("Year fraction calculation should succeed");
        let t3 = dc
            .year_fraction(
                base_date,
                base_date + time::Duration::days(365 * 3),
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("Year fraction calculation should succeed");
        let t5 = dc
            .year_fraction(
                base_date,
                base_date + time::Duration::days(365 * 5),
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("Year fraction calculation should succeed");
        assert!((hazard.quoted_spread_bp(t1, ParInterp::Linear) - 50.0).abs() < 1e-6);
        assert!((hazard.quoted_spread_bp(t3, ParInterp::Linear) - 75.0).abs() < 1e-6);
        assert!((hazard.quoted_spread_bp(t5, ParInterp::Linear) - 100.0).abs() < 1e-6);
        // Interpolated in-between (between 1Y and 3Y)
        let mid = hazard.quoted_spread_bp((t1 + t3) * 0.5, ParInterp::Linear);
        assert!(mid > 50.0 && mid < 75.0);

        // Residuals are small
        for v in report.residuals.values() {
            assert!(v.abs() <= 1e-6);
        }
    }

    #[test]
    fn hazard_calibration_errors_on_empty_quotes() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();
        let calibrator = HazardCurveCalibrator::new(
            "AAPL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );
        let market_context = MarketContext::new().insert_discount(disc);
        let empty: Vec<CreditQuote> = vec![];
        let res = calibrator.calibrate(&empty, &market_context);
        assert!(res.is_err());
    }

    #[test]
    fn test_upfront_cds_quote_support() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        // Test with upfront quote
        let upfront_quote = vec![CreditQuote::CDSUpfront {
            entity: "DISTRESSED".to_string(),
            maturity: base_date + time::Duration::days(365),
            upfront_pct: 5.0,         // 5% upfront
            running_spread_bp: 300.0, // 300bp running
            recovery_rate: 0.25,      // Lower recovery for distressed
            currency: Currency::USD,
        }];

        // Use relaxed tolerance for upfront CDS calibration (complex pricing model)
        let config = crate::calibration::CalibrationConfig::default().with_tolerance(1e-4);
        let calibrator = HazardCurveCalibrator::new(
            "DISTRESSED",
            Seniority::Senior,
            0.25,
            base_date,
            Currency::USD,
            "USD-OIS",
        )
        .with_config(config);
        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&upfront_quote, &market_context);

        // Should succeed and handle upfront quote properly
        assert!(result.is_ok());
        let (_curve, report) = result.expect("Hazard curve calibration should succeed in test");
        assert!(
            report.success,
            "Calibration failed: {}",
            report.convergence_reason
        );

        // Check that residual key indicates upfront quote
        let upfront_residual_key = format!("CDS-UPFRONT-{}", base_date + time::Duration::days(365));
        assert!(report.residuals.contains_key(&upfront_residual_key));
    }

    #[test]
    fn test_default_discount_curve_id_helpers() {
        // Test currency-based discount curve ID generation
        assert_eq!(
            HazardCurveCalibrator::default_discount_curve_id(Currency::USD),
            CurveId::new("USD-OIS")
        );
        assert_eq!(
            HazardCurveCalibrator::default_discount_curve_id(Currency::EUR),
            CurveId::new("EUR-OIS")
        );
        assert_eq!(
            HazardCurveCalibrator::default_discount_curve_id(Currency::GBP),
            CurveId::new("GBP-OIS")
        );

        // Test convenience constructor
        let calibrator = HazardCurveCalibrator::new_with_default_discount(
            "TEST",
            Seniority::Senior,
            0.40,
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
            Currency::JPY,
        );
        assert_eq!(calibrator.discount_curve_id, CurveId::new("JPY-OIS"));
    }

    // ========== Market Standards Edge Case Tests ==========

    #[test]
    fn test_recovery_rate_mismatch_validation() {
        // Test that calibrator rejects quotes with inconsistent recovery rates
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        // Calibrator expects 40% recovery
        let calibrator = HazardCurveCalibrator::new(
            "MISMATCH",
            Seniority::Senior,
            0.40, // Calibrator recovery
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        // Quote has 25% recovery (mismatch > RECOVERY_TOLERANCE)
        let mismatched_quotes = vec![CreditQuote::CDS {
            entity: "MISMATCH".to_string(),
            maturity: base_date + time::Duration::days(365),
            spread_bp: 100.0,
            recovery_rate: 0.25, // Differs by 15pp from calibrator
            currency: Currency::USD,
        }];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&mismatched_quotes, &market_context);

        // Should fail with validation error
        assert!(
            result.is_err(),
            "Calibration should fail when quote recovery differs from calibrator"
        );
        let err_msg = format!("{:?}", result.err());
        assert!(
            err_msg.contains("Recovery rate mismatch"),
            "Error should mention recovery rate mismatch: {}",
            err_msg
        );
    }

    #[test]
    fn test_recovery_rate_within_tolerance() {
        // Test that small recovery rate differences within tolerance are accepted
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "WITHIN_TOL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        // Quote recovery within RECOVERY_TOLERANCE (0.01 = 1pp)
        let quotes = vec![CreditQuote::CDS {
            entity: "WITHIN_TOL".to_string(),
            maturity: base_date + time::Duration::days(365),
            spread_bp: 100.0,
            recovery_rate: 0.405, // 0.5pp difference - within tolerance
            currency: Currency::USD,
        }];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&quotes, &market_context);

        // Should succeed
        assert!(
            result.is_ok(),
            "Calibration should succeed when recovery rates are within tolerance: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_distressed_credit_high_spreads() {
        // Test calibration with high spreads typical of distressed credits (B/CCC rated)
        // Note: Extremely high spreads (>2000bp) can cause numerical issues in CDS pricing
        // due to survival probabilities approaching zero, which is realistic market behavior.
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "DISTRESSED",
            Seniority::Senior,
            0.25, // Lower recovery for distressed
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        // High but realistic distressed spreads: 500bp, 750bp, 1000bp
        // These represent B/CCC-rated credits with meaningful default probability
        let distressed_quotes = vec![
            CreditQuote::CDS {
                entity: "DISTRESSED".to_string(),
                maturity: base_date + time::Duration::days(365),
                spread_bp: 500.0,
                recovery_rate: 0.25,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "DISTRESSED".to_string(),
                maturity: base_date + time::Duration::days(365 * 3),
                spread_bp: 750.0,
                recovery_rate: 0.25,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "DISTRESSED".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 1000.0,
                recovery_rate: 0.25,
                currency: Currency::USD,
            },
        ];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&distressed_quotes, &market_context);

        // Should succeed with adaptive grid handling high lambdas
        assert!(
            result.is_ok(),
            "Calibration should succeed for distressed credits: {:?}",
            result.err()
        );

        let (curve, report) = result.expect("Should succeed");
        assert!(report.success);

        // Verify hazard rates are positive and reasonable for distressed credits
        // For 500bp spread with 25% recovery: λ ≈ 500/10000 / 0.75 ≈ 0.067 (6.7%)
        for (t, lambda) in curve.knot_points() {
            assert!(
                lambda > 0.0,
                "Hazard rate at t={} must be positive, got {}",
                t,
                lambda
            );
            // High but not physically unreasonable - distressed credits can have 10-20% hazard
            assert!(
                lambda < 0.50,
                "Hazard rate at t={} should be < 50%, got {:.2}%",
                t,
                lambda * 100.0
            );
        }

        // Verify survival probability decreases significantly over time
        let sp_1y = curve.sp(1.0);
        let sp_5y = curve.sp(5.0);
        assert!(
            sp_1y > sp_5y,
            "Survival probability should decrease: SP(1Y)={} > SP(5Y)={}",
            sp_1y,
            sp_5y
        );
        // For 1000bp spread with 25% recovery over 5Y, expect significant default probability
        assert!(
            sp_5y < 0.80,
            "5Y survival probability for distressed should be < 80%, got {:.1}%",
            sp_5y * 100.0
        );
    }

    #[test]
    fn test_tight_spreads_investment_grade() {
        // Test calibration with very tight spreads (investment grade)
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "AAA_CORP",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        // Very tight spreads: 5bp, 10bp, 15bp
        let tight_quotes = vec![
            CreditQuote::CDS {
                entity: "AAA_CORP".to_string(),
                maturity: base_date + time::Duration::days(365),
                spread_bp: 5.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "AAA_CORP".to_string(),
                maturity: base_date + time::Duration::days(365 * 3),
                spread_bp: 10.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
            CreditQuote::CDS {
                entity: "AAA_CORP".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 15.0,
                recovery_rate: 0.40,
                currency: Currency::USD,
            },
        ];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&tight_quotes, &market_context);

        // Should succeed with adaptive grid handling low lambdas
        assert!(
            result.is_ok(),
            "Calibration should succeed for tight spreads: {:?}",
            result.err()
        );

        let (curve, report) = result.expect("Should succeed");
        assert!(report.success);

        // Verify hazard rates are small but positive
        for (t, lambda) in curve.knot_points() {
            assert!(
                lambda > 0.0,
                "Hazard rate at t={} must be positive, got {}",
                t,
                lambda
            );
            // Very tight spreads should produce very small lambda
            assert!(
                lambda < 0.01,
                "Hazard rate at t={} for IG should be < 1%, got {:.4}%",
                t,
                lambda * 100.0
            );
        }
    }

    #[test]
    fn test_convention_day_count_consistency() {
        // Test that Asian convention uses correct day count (ACT/365F)
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "ASIA_CORP",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::JPY, // Should use IsdaAs convention
            "USD-OIS",
        );

        assert_eq!(
            calibrator.convention,
            CDSConvention::IsdaAs,
            "JPY should default to Asian convention"
        );

        let quotes = vec![CreditQuote::CDS {
            entity: "ASIA_CORP".to_string(),
            maturity: base_date + time::Duration::days(365),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            currency: Currency::JPY,
        }];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&quotes, &market_context);

        assert!(result.is_ok());
        let (curve, _) = result.expect("Should succeed");

        // Verify curve day count matches convention
        assert_eq!(
            curve.day_count(),
            finstack_core::dates::DayCount::Act365F,
            "Asian convention curve should use ACT/365F"
        );
    }

    #[test]
    fn test_determinism_multiple_calibrations() {
        // Test that repeated calibrations produce identical results
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();
        let quotes = test_cds_quotes();

        let calibrator = HazardCurveCalibrator::new(
            "AAPL",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        let market_context = MarketContext::new().insert_discount(disc);

        // Calibrate multiple times
        let (curve1, _) = calibrator
            .calibrate(&quotes, &market_context)
            .expect("First calibration should succeed");
        let (curve2, _) = calibrator
            .calibrate(&quotes, &market_context)
            .expect("Second calibration should succeed");
        let (curve3, _) = calibrator
            .calibrate(&quotes, &market_context)
            .expect("Third calibration should succeed");

        // Compare survival probabilities at multiple points
        for t in [0.5, 1.0, 2.0, 3.0, 4.0, 5.0] {
            let sp1 = curve1.sp(t);
            let sp2 = curve2.sp(t);
            let sp3 = curve3.sp(t);

            assert!(
                (sp1 - sp2).abs() < 1e-15,
                "Determinism violated at t={}: sp1={} sp2={}",
                t,
                sp1,
                sp2
            );
            assert!(
                (sp2 - sp3).abs() < 1e-15,
                "Determinism violated at t={}: sp2={} sp3={}",
                t,
                sp2,
                sp3
            );
        }
    }

    #[test]
    fn test_adaptive_grid_coverage() {
        // Test that adaptive_grid produces reasonable grids for various initial guesses

        // Investment grade: λ ≈ 0.001 (60bp / 60% LGD)
        let ig_grid = HazardCurveCalibrator::adaptive_grid(0.001);
        assert!(
            ig_grid[0] < 0.001,
            "Grid should extend below initial guess for IG"
        );
        assert!(
            ig_grid[15] > 0.001,
            "Grid should extend above initial guess for IG"
        );

        // High yield: λ ≈ 0.05 (300bp / 60% LGD)
        let hy_grid = HazardCurveCalibrator::adaptive_grid(0.05);
        assert!(
            hy_grid[0] < 0.05,
            "Grid should extend below initial guess for HY"
        );
        assert!(
            hy_grid[15] > 0.05,
            "Grid should extend above initial guess for HY"
        );

        // Distressed: λ ≈ 0.5 (3000bp / 60% LGD)
        let distressed_grid = HazardCurveCalibrator::adaptive_grid(0.5);
        assert!(
            distressed_grid[0] < 0.5,
            "Grid should extend below initial guess for distressed"
        );
        assert!(
            distressed_grid[15] >= 0.5,
            "Grid should extend to or above initial guess for distressed"
        );

        // Verify all grids are strictly increasing
        for grid in [&ig_grid, &hy_grid, &distressed_grid] {
            for i in 1..grid.len() {
                assert!(
                    grid[i] > grid[i - 1],
                    "Grid must be strictly increasing: {}[{}]={} <= {}[{}]={}",
                    "grid",
                    i,
                    grid[i],
                    "grid",
                    i - 1,
                    grid[i - 1]
                );
            }
        }
    }

    #[test]
    fn test_negative_spread_rejection() {
        // Test that negative spreads are rejected
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "NEGATIVE",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        let negative_quotes = vec![CreditQuote::CDS {
            entity: "NEGATIVE".to_string(),
            maturity: base_date + time::Duration::days(365),
            spread_bp: -50.0, // Invalid negative spread
            recovery_rate: 0.40,
            currency: Currency::USD,
        }];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&negative_quotes, &market_context);

        assert!(
            result.is_err(),
            "Calibration should fail for negative spreads"
        );
    }

    #[test]
    fn test_zero_spread_rejection() {
        // Test that zero spreads are rejected
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = test_discount_curve();

        let calibrator = HazardCurveCalibrator::new(
            "ZERO",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        let zero_quotes = vec![CreditQuote::CDS {
            entity: "ZERO".to_string(),
            maturity: base_date + time::Duration::days(365),
            spread_bp: 0.0, // Invalid zero spread
            recovery_rate: 0.40,
            currency: Currency::USD,
        }];

        let market_context = MarketContext::new().insert_discount(disc);
        let result = calibrator.calibrate(&zero_quotes, &market_context);

        assert!(result.is_err(), "Calibration should fail for zero spreads");
    }

    #[test]
    fn test_with_convention_builder() {
        // Test the with_convention builder method
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let calibrator = HazardCurveCalibrator::new(
            "TEST",
            Seniority::Senior,
            0.40,
            base_date,
            Currency::USD,
            "USD-OIS",
        )
        .with_convention(CDSConvention::IsdaEu);

        assert_eq!(
            calibrator.convention,
            CDSConvention::IsdaEu,
            "Convention should be overridden to EU"
        );
    }
}

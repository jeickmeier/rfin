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

use crate::calibration::quotes::CreditQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator};
use crate::constants::time as time_constants;
use crate::instruments::cds::pricer::CDSPricer;
use crate::instruments::cds::{CDSConvention, PayReceive};
use finstack_core::config::FinstackConfig;
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

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> finstack_core::Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
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

        // Get day count from convention for the curve
        let curve_day_count = self.convention.day_count();

        // Extract CDS quotes for this entity and sort by maturity
        let mut cds_quotes: Vec<(
            finstack_core::dates::Date,
            f64,
            Option<f64>,
            &crate::calibration::quotes::InstrumentConventions,
        )> = quotes
            .iter()
            .filter_map(|q| match q {
                CreditQuote::CDS {
                    entity,
                    maturity,
                    spread_bp,
                    conventions,
                    ..
                } if entity == &self.entity => Some((*maturity, *spread_bp, None, conventions)),
                CreditQuote::CDSUpfront {
                    entity,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                    conventions,
                    ..
                } if entity == &self.entity => Some((
                    *maturity,
                    *running_spread_bp,
                    Some(*upfront_pct),
                    conventions,
                )),
                _ => None,
            })
            .collect();

        if cds_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        // Validate all spreads are positive (zero or negative spreads are invalid)
        for (maturity, spread_bp, _, _) in &cds_quotes {
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

        for (maturity, market_spread_bp, upfront_pct_opt, convention_overrides) in &cds_quotes {
            // Resolve conventions (override > calibrator default)
            let instrument_day_count = convention_overrides
                .day_count
                .unwrap_or_else(|| self.convention.day_count());
            let freq = convention_overrides
                .payment_frequency
                .unwrap_or_else(|| self.convention.frequency());
            let bdc = convention_overrides
                .business_day_convention
                .unwrap_or_else(|| self.convention.business_day_convention());
            let calendar_id = convention_overrides
                .effective_payment_calendar_id()
                .unwrap_or_else(|| self.convention.default_calendar());
            let settlement_days = convention_overrides
                .settlement_days
                .unwrap_or(self.convention.settlement_delay() as i32)
                as u16;

            // Time axis using CURVE day count (must be consistent for all knots)
            let tenor_years = curve_day_count
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

            let premium_spec = crate::instruments::cds::PremiumLegSpec {
                start: self.base_date,
                end: *maturity,
                freq,
                stub: self.convention.stub_convention(),
                bdc,
                calendar_id: Some(calendar_id.to_string()),
                dc: instrument_day_count,
                spread_bp: *market_spread_bp,
                discount_curve_id: finstack_core::types::CurveId::new(CALIB_DISC_ID),
            };

            let protection_spec = crate::instruments::cds::ProtectionLegSpec {
                credit_curve_id: finstack_core::types::CurveId::new(CALIB_HAZARD_ID),
                recovery_rate: self.recovery_rate,
                settlement_delay: settlement_days,
            };

            let cds = crate::instruments::cds::CreditDefaultSwapBuilder::new()
                .id(format!("CALIB_CDS_{}", maturity).into())
                .notional(Money::new(10_000_000.0, self.currency))
                .side(PayReceive::PayFixed)
                .convention(self.convention)
                .premium(premium_spec)
                .protection(protection_spec)
                .pricing_overrides(crate::instruments::PricingOverrides::default())
                .attributes(crate::instruments::common::traits::Attributes::new())
                .build()
                .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

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
            .day_count(curve_day_count)
            .base_date(self.base_date)
            .knots(hazard_knots.into_vec())
            .par_spreads(par_knots.into_vec())
            .par_interp(self.par_interp)
            .build()?;

        // Validate the calibrated hazard curve (honor config.validation + validation_mode).
        use crate::calibration::validation::CurveValidator;
        let mut validation_status = "passed";
        let mut validation_error: Option<String> = None;
        if let Err(e) = curve.validate(&self.config.validation) {
            validation_status = "failed";
            validation_error = Some(e.to_string());
            match self.config.validation_mode {
                crate::calibration::ValidationMode::Warn => {
                    tracing::warn!(
                        entity = %self.entity,
                        error = %e,
                        "Calibrated hazard curve failed validation (continuing due to Warn mode)"
                    );
                }
                crate::calibration::ValidationMode::Error => {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Calibrated hazard curve for {} failed validation: {}",
                            self.entity, e
                        ),
                        category: "hazard_curve_validation".to_string(),
                    });
                }
            }
        }

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
        .with_metadata("validation", validation_status)
        .with_validation_result(validation_status == "passed", validation_error);

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

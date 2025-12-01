//! Base correlation curve calibration from CDS tranche quotes.
//!
//! Implements market-standard base correlation bootstrapping using the
//! one-factor Gaussian Copula model and equity tranche decomposition.
//!
//! # Methodology
//!
//! Base correlation calibration follows the methodology established by
//! McGinty et al. (2004) "Introducing Base Correlations" and standardized
//! in the ISDA CDX/iTraxx market conventions:
//!
//! 1. Sort tranches by detachment point (equity to senior)
//! 2. For each tranche \[A, D\], solve for ρ(D) such that:
//!    `Price([A, D]) = Price([0, D], ρ(D)) - Price([0, A], ρ(A))`
//! 3. Use previously solved correlations for \[0, A\] pricing
//!
//! # References
//!
//! - McGinty, L., Beinstein, E., et al. (2004). "Introducing Base Correlations."
//!   JPMorgan Credit Derivatives Strategy.
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit Derivatives*.
//!   Wiley Finance. Chapters 6-8.
//! - ISDA (2009). Big Bang Protocol for CDS standardization.

use crate::calibration::quote::CreditQuote;
use crate::calibration::{
    CalibrationConfig, CalibrationReport, Calibrator, SolverConfig, SolverKind,
};
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use finstack_core::math::Solver;
use ordered_float::OrderedFloat;

use finstack_core::dates::{next_cds_date, BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::prelude::*;

use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Interpolation method for base correlation curves.
///
/// Controls how base correlations are interpolated between calibrated
/// detachment points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CorrelationInterp {
    /// Linear interpolation between detachment points (market standard)
    #[default]
    Linear,
}

/// Minimum correlation bound (0.1% to avoid numerical issues near zero)
const MIN_CORRELATION: f64 = 0.001;

/// Maximum correlation bound (99.9% to avoid numerical issues near unity)
const MAX_CORRELATION: f64 = 0.999;

/// Default initial correlation guess for equity tranches
const INITIAL_CORRELATION_GUESS: f64 = 0.30;

/// Correlation step size for monotonic assumption in initial guess
const CORRELATION_STEP: f64 = 0.05;

/// Maximum correlation for monotonic extrapolation of initial guess
const MAX_MONOTONIC_CORRELATION: f64 = 0.90;

/// Minimum correlation for fallback bracketing
const MIN_BRACKET_CORRELATION: f64 = 0.02;

/// Maximum correlation for fallback bracketing
const MAX_BRACKET_CORRELATION: f64 = 0.98;

/// Finite penalty value for objective function failures.
///
/// Using a large finite value helps solvers behave predictably
/// and provides meaningful diagnostics when calibration fails.
const CALIBRATION_PENALTY: f64 = 1e12;

/// Base correlation curve calibrator.
///
/// Calibrates a base correlation curve from CDS tranche market quotes
/// using sequential bootstrapping with the Gaussian Copula model.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::BaseCorrelationCalibrator;
/// use finstack_valuations::calibration::Calibrator;
///
/// let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date)
///     .with_config(config)
///     .with_discount_curve_id("USD-OIS");
///
/// let (curve, report) = calibrator.calibrate(&quotes, &market_context)?;
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseCorrelationCalibrator {
    /// Index identifier (e.g., "CDX.NA.IG.42")
    pub index_id: String,
    /// Index series number
    pub series: u16,
    /// Maturity for correlation curve (e.g., 5 years)
    pub maturity_years: f64,
    /// Base date for calibration
    pub base_date: Date,
    /// Discount curve identifier used for tranche PVs
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Standard detachment points to calibrate
    pub detachment_points: Vec<f64>,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Interpolation used for base correlation between detachment points
    pub corr_interp: CorrelationInterp,
    /// Whether to use IMM dates for maturity calculation (standard for CDX/iTraxx)
    pub use_imm_dates: bool,
}

impl BaseCorrelationCalibrator {
    /// Create a new base correlation calibrator.
    ///
    /// # Arguments
    ///
    /// * `index_id` - Index identifier (e.g., "CDX.NA.IG.42")
    /// * `series` - Index series number
    /// * `maturity_years` - Target maturity in years (e.g., 5.0 for 5Y)
    /// * `base_date` - Calibration date
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        maturity_years: f64,
        base_date: Date,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            maturity_years,
            base_date,
            // Default to common OIS discounting for USD
            discount_curve_id: finstack_core::types::CurveId::from("USD-OIS"),
            // Standard market detachment points for CDX.IG
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
            corr_interp: CorrelationInterp::Linear,
            // Default to IMM dates for standard indices
            use_imm_dates: true,
        }
    }

    /// Set custom detachment points.
    ///
    /// Points will be sorted internally before calibration.
    pub fn with_detachment_points(mut self, points: Vec<f64>) -> Self {
        self.detachment_points = points;
        self
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the discount curve identifier used when pricing synthetic tranches.
    pub fn with_discount_curve_id(
        mut self,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        self.discount_curve_id = discount_curve_id.into();
        self
    }

    /// Set interpolation method for base correlation between detachment points.
    pub fn with_corr_interp(mut self, interp: CorrelationInterp) -> Self {
        self.corr_interp = interp;
        self
    }

    /// Set whether to use IMM dates for maturity calculation.
    ///
    /// When enabled (default), maturities snap to CDS IMM dates
    /// (20th of Mar/Jun/Sep/Dec) as per ISDA conventions.
    pub fn with_imm_dates(mut self, use_imm: bool) -> Self {
        self.use_imm_dates = use_imm;
        self
    }

    /// Bootstrap base correlation curve from tranche quotes using sequential calibration.
    fn bootstrap_curve<S: Solver>(
        &self,
        quotes: &[CreditQuote],
        solver: &S,
        market_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        use crate::instruments::cds_tranche::pricer::CDSTranchePricer;

        // Filter and extract CDS tranche quotes, keeping only the requested index
        let mut tranche_quotes: Vec<_> = quotes
            .iter()
            .filter_map(|q| {
                if let CreditQuote::CDSTranche {
                    index,
                    attachment,
                    detachment,
                    maturity,
                    upfront_pct,
                    running_spread_bp,
                } = q
                {
                    if index == &self.index_id {
                        Some((
                            *attachment,
                            *detachment,
                            *maturity,
                            *upfront_pct,
                            *running_spread_bp,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if tranche_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("No CDS tranche quotes found for index '{}'", self.index_id),
                },
            ));
        }

        // Validate no NaN/Inf values in detachment points before sorting
        for (attach, detach, _, _, _) in &tranche_quotes {
            if !attach.is_finite() || !detach.is_finite() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        // Sort by detachment point for sequential bootstrapping (equity to senior)
        tranche_quotes.sort_by(|a, b| OrderedFloat(a.1).cmp(&OrderedFloat(b.1)));

        // Validate tranche quote structure
        for (attach, detach, _, upfront, spread) in &tranche_quotes {
            if *attach >= *detach {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Invalid tranche: attachment ({:.2}%) >= detachment ({:.2}%)",
                        attach, detach
                    ),
                    category: "base_correlation_input".to_string(),
                });
            }
            if *attach < 0.0 || *detach <= 0.0 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::NegativeValue,
                ));
            }
            if !upfront.is_finite() || !spread.is_finite() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }

        let mut solved_correlations: Vec<(f64, f64)> = Vec::with_capacity(tranche_quotes.len());
        let mut residuals = BTreeMap::new();
        let mut total_function_evaluations: usize = 0;
        let pricing_model = CDSTranchePricer::new();
        let num_tranche_quotes = tranche_quotes.len();

        // Sequential bootstrap from equity to senior tranches
        for (index, (attach_pct, detach_pct, _maturity, upfront_pct, running_spread_bp)) in
            tranche_quotes.into_iter().enumerate()
        {
            // Create synthetic tranche for this quote
            let synthetic_tranche =
                self.create_synthetic_tranche(attach_pct, detach_pct, running_spread_bp)?;

            // Target upfront value from market quote (convert % to absolute)
            let target_upfront = upfront_pct / 100.0 * synthetic_tranche.notional.amount();

            // Determine initial guess using monotonic seeding strategy
            let initial_guess = self.compute_initial_guess(&solved_correlations);

            // Create shared state for counting function evaluations
            let eval_counter = Cell::new(0usize);

            // Build objective function with evaluation counting
            // We need a closure that can be called multiple times
            let make_objective = || {
                |trial_correlation: f64| -> f64 {
                    eval_counter.set(eval_counter.get() + 1);

                    // Clamp trial correlation to valid range
                    let clamped_trial = trial_correlation.clamp(MIN_CORRELATION, MAX_CORRELATION);

                    // Build temporary correlation curve including solved points
                    let mut temp_corr_points = Vec::with_capacity(solved_correlations.len() + 2);
                    temp_corr_points.extend_from_slice(&solved_correlations);
                    temp_corr_points.push((detach_pct, clamped_trial));

                    // Ensure minimum curve requirements (need at least 2 points)
                    if temp_corr_points.len() < 2 {
                        // Add a second point for curve construction
                        temp_corr_points.push((detach_pct + 10.0, clamped_trial));
                    }

                    let temp_base_corr_curve =
                        match BaseCorrelationCurve::builder("TEMP_CALIB_CORR")
                            .knots(temp_corr_points)
                            .build()
                        {
                            Ok(curve) => Arc::new(curve),
                            Err(_) => return CALIBRATION_PENALTY,
                        };

                    // Update market context with trial correlation curve
                    let mut temp_market_ctx = market_context.clone();
                    if !temp_market_ctx.update_base_correlation_curve(
                        &synthetic_tranche.credit_index_id,
                        temp_base_corr_curve,
                    ) {
                        return CALIBRATION_PENALTY;
                    }

                    // Price tranche and compute residual
                    match pricing_model.price_tranche(
                        &synthetic_tranche,
                        &temp_market_ctx,
                        self.base_date,
                    ) {
                        Ok(pv) => pv.amount() - target_upfront,
                        Err(_) => CALIBRATION_PENALTY,
                    }
                }
            };

            // Solve for correlation with primary method
            let solve_result = solver.solve(make_objective(), initial_guess);

            // Handle solve result with fallback bracketing if needed
            let solved_corr = match solve_result {
                Ok(corr) => corr,
                Err(_) => {
                    // Fallback: try bracketed search across full correlation range
                    self.bracketed_correlation_search(&make_objective(), initial_guess)?
                }
            };

            // Capture function evaluations for this tranche
            let tranche_evals = eval_counter.get();
            total_function_evaluations += tranche_evals;

            // Clamp to valid bounds
            let clamped_corr = solved_corr.clamp(MIN_CORRELATION, MAX_CORRELATION);

            // Calculate final residual for reporting
            let final_residual = make_objective()(clamped_corr);

            // Validate monotonicity (base correlation should increase with detachment)
            if let Some(&(_, prev_corr)) = solved_correlations.last() {
                if clamped_corr < prev_corr - 1e-6 {
                    tracing::warn!(
                        "Base correlation non-monotonic at {:.1}%: {:.4} < {:.4} (previous)",
                        detach_pct,
                        clamped_corr,
                        prev_corr
                    );
                }
            }

            solved_correlations.push((detach_pct, clamped_corr));
            let key = format!("tranche_{}_{}_{}", index, attach_pct, detach_pct);
            residuals.insert(key, final_residual);
        }

        if solved_correlations.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "No tranches successfully calibrated".to_string(),
                category: "base_correlation_empty".to_string(),
            });
        }

        // Build final base correlation curve
        let final_curve = BaseCorrelationCurve::builder("CALIBRATED_BASE_CORR")
            .knots(solved_correlations.clone())
            .build()?;

        // Validate the calibrated base correlation curve
        use crate::calibration::validation::{CurveValidator, ValidationConfig};
        final_curve
            .validate(&ValidationConfig::default())
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Calibrated base correlation curve failed validation: {}", e),
                category: "base_correlation_validation".to_string(),
            })?;

        // Build comprehensive calibration report
        let solver_config = self.build_solver_config();
        let report = CalibrationReport::for_type_with_tolerance(
            "base_correlation",
            residuals,
            total_function_evaluations,
            self.config.tolerance,
        )
        .with_metadata("calibrated_tranches", num_tranche_quotes.to_string())
        .with_metadata("corr_interp", format!("{:?}", self.corr_interp))
        .with_metadata("index_id", self.index_id.clone())
        .with_metadata("maturity_years", self.maturity_years.to_string())
        .with_metadata("use_imm_dates", self.use_imm_dates.to_string())
        .with_metadata(
            "function_evaluations",
            total_function_evaluations.to_string(),
        )
        .with_metadata("validation", "passed")
        .with_solver_config(solver_config);

        Ok((final_curve, report))
    }

    /// Compute initial guess for correlation using monotonic seeding strategy.
    ///
    /// For the first tranche, uses a reasonable starting point for equity tranches.
    /// For subsequent tranches, starts slightly above the previous solved correlation
    /// (assuming typical monotonically increasing base correlation).
    fn compute_initial_guess(&self, solved_correlations: &[(f64, f64)]) -> f64 {
        if solved_correlations.is_empty() {
            INITIAL_CORRELATION_GUESS
        } else {
            let (_, last_correlation) = solved_correlations
                .last()
                .expect("solved_correlations checked non-empty");
            (*last_correlation + CORRELATION_STEP).min(MAX_MONOTONIC_CORRELATION)
        }
    }

    /// Fallback bracketed search when primary solver fails.
    ///
    /// Scans across the valid correlation range to find a root, handling
    /// edge cases like inverted correlation curves or numerical instability.
    fn bracketed_correlation_search<F>(&self, objective: &F, initial_guess: f64) -> Result<f64>
    where
        F: Fn(f64) -> f64,
    {
        // Try several bracketing strategies
        let bracket_points = [
            (MIN_BRACKET_CORRELATION, initial_guess),
            (initial_guess, MAX_BRACKET_CORRELATION),
            (MIN_BRACKET_CORRELATION, MAX_BRACKET_CORRELATION),
            (0.10, 0.50),
            (0.30, 0.70),
            (0.50, 0.90),
        ];

        for (lo, hi) in bracket_points {
            let f_lo = objective(lo);
            let f_hi = objective(hi);

            // Check if we have a bracket (sign change)
            if f_lo * f_hi < 0.0 && f_lo.is_finite() && f_hi.is_finite() {
                // Bisection search within bracket
                let mut a = lo;
                let mut b = hi;
                let mut fa = f_lo;

                for _ in 0..100 {
                    let mid = 0.5 * (a + b);
                    let f_mid = objective(mid);

                    if f_mid.abs() < self.config.tolerance || (b - a) < 1e-10 {
                        return Ok(mid);
                    }

                    if fa * f_mid < 0.0 {
                        b = mid;
                    } else {
                        a = mid;
                        fa = f_mid;
                    }
                }

                return Ok(0.5 * (a + b));
            }
        }

        // Final fallback: return initial guess with warning
        tracing::warn!(
            "Base correlation calibration: bracketed search failed, using initial guess {:.4}",
            initial_guess
        );
        Ok(initial_guess.clamp(MIN_CORRELATION, MAX_CORRELATION))
    }

    /// Build solver configuration for report persistence.
    fn build_solver_config(&self) -> SolverConfig {
        match self.config.solver_kind {
            SolverKind::Newton => SolverConfig::Newton {
                tolerance: self.config.tolerance,
                max_iterations: self.config.max_iterations,
                fd_step: 1e-8,
                min_derivative: 1e-14,
            },
            SolverKind::Brent | SolverKind::LevenbergMarquardt => SolverConfig::Brent {
                tolerance: self.config.tolerance,
                max_iterations: self.config.max_iterations,
                bracket_expansion: 1.6,
                initial_bracket_size: None,
            },
        }
    }

    /// Create synthetic CDS tranche for pricing during calibration.
    ///
    /// Uses IMM date conventions when `use_imm_dates` is enabled,
    /// aligning with ISDA Big Bang Protocol (2009) standardization.
    fn create_synthetic_tranche(
        &self,
        attach_pct: f64,
        detach_pct: f64,
        running_spread_bp: f64,
    ) -> Result<CdsTranche> {
        // Calculate maturity date using proper IMM conventions if enabled
        let maturity = if self.use_imm_dates {
            self.calculate_imm_maturity()?
        } else {
            // Simple month-based calculation as fallback
            let months_to_add = (self.maturity_years * 12.0).round() as i32;
            finstack_core::dates::add_months(self.base_date, months_to_add)
        };

        let id = finstack_core::types::InstrumentId::new(
            format!("CALIB_TRANCHE_{:.1}_{:.1}", attach_pct, detach_pct).replace('.', "_"),
        );

        CdsTranche::builder()
            .id(id)
            .index_name(self.index_id.to_owned())
            .series(self.series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .maturity(maturity)
            .running_coupon_bp(running_spread_bp)
            .payment_frequency(Frequency::quarterly())
            .day_count(DayCount::Act360)
            .business_day_convention(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .discount_curve_id(self.discount_curve_id.to_owned())
            .credit_index_id(finstack_core::types::CurveId::new(self.index_id.to_owned()))
            .side(TrancheSide::SellProtection)
            .effective_date_opt(None)
            .accumulated_loss(0.0)
            .standard_imm_dates(self.use_imm_dates)
            .build()
    }

    /// Calculate maturity date using CDS IMM conventions.
    ///
    /// CDS tranches mature on IMM dates: 20th of Mar/Jun/Sep/Dec.
    /// This finds the IMM date closest to base_date + maturity_years.
    fn calculate_imm_maturity(&self) -> Result<Date> {
        // Calculate approximate target date
        let months_to_add = (self.maturity_years * 12.0).round() as i32;
        let approximate_maturity = finstack_core::dates::add_months(self.base_date, months_to_add);

        // Snap to next CDS IMM date (20th of Mar/Jun/Sep/Dec)
        // If we're already past the approximate date, step back one day to catch current IMM
        let search_start = if approximate_maturity.day() >= 20 {
            // We might be past this quarter's IMM, so go back slightly
            finstack_core::dates::add_months(approximate_maturity, -3)
        } else {
            finstack_core::dates::add_months(approximate_maturity, -1)
        };

        // Find the next CDS date from search_start
        let mut imm_date = next_cds_date(search_start);

        // If we overshot significantly, try the previous IMM date
        let days_diff = (imm_date - approximate_maturity).whole_days();
        if days_diff > 45 {
            // We went too far forward, try going back one quarter
            let earlier_search = finstack_core::dates::add_months(search_start, -3);
            let earlier_imm = next_cds_date(earlier_search);
            if (earlier_imm - approximate_maturity).whole_days().abs()
                < (imm_date - approximate_maturity).whole_days().abs()
            {
                imm_date = earlier_imm;
            }
        }

        Ok(imm_date)
    }
}

impl Calibrator<CreditQuote, BaseCorrelationCurve> for BaseCorrelationCalibrator {
    fn calibrate(
        &self,
        instruments: &[CreditQuote],
        base_context: &MarketContext,
    ) -> Result<(BaseCorrelationCurve, CalibrationReport)> {
        let solver = crate::calibration::create_simple_solver(&self.config);
        self.bootstrap_curve(instruments, &solver, base_context)
    }
}

/// Multi-expiry base correlation surface calibrator.
///
/// Calibrates base correlation curves for multiple maturities and
/// builds a correlation surface indexed by maturity.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::BaseCorrelationSurfaceCalibrator;
///
/// let calibrator = BaseCorrelationSurfaceCalibrator::new(
///     "CDX.NA.IG.42",
///     42,
///     base_date,
///     vec![3.0, 5.0, 7.0, 10.0],  // Target maturities in years
/// );
///
/// let (curves, report) = calibrator.calibrate_surface(&quotes, &market_context)?;
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseCorrelationSurfaceCalibrator {
    /// Index identifier
    pub index_id: String,
    /// Index series
    pub series: u16,
    /// Base date
    pub base_date: Date,
    /// Target maturities in years
    pub target_maturities: Vec<f64>,
    /// Standard detachment points
    pub detachment_points: Vec<f64>,
    /// Calibration configuration
    pub config: CalibrationConfig,
    /// Discount curve identifier used for tranche PVs
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Interpolation used for base correlation between detachment points
    pub corr_interp: CorrelationInterp,
    /// Day count used to map tranche maturities to years for grouping
    pub time_dc: DayCount,
    /// Whether to use IMM dates for maturity calculation
    pub use_imm_dates: bool,
}

impl BaseCorrelationSurfaceCalibrator {
    /// Create a new surface calibrator.
    pub fn new(
        index_id: impl Into<String>,
        series: u16,
        base_date: Date,
        target_maturities: Vec<f64>,
    ) -> Self {
        Self {
            index_id: index_id.into(),
            series,
            base_date,
            target_maturities,
            detachment_points: vec![3.0, 7.0, 10.0, 15.0, 30.0],
            config: CalibrationConfig::default(),
            discount_curve_id: finstack_core::types::CurveId::from("USD-OIS"),
            corr_interp: CorrelationInterp::Linear,
            time_dc: DayCount::Act360, // Use Act360 to align with CDS conventions
            use_imm_dates: true,
        }
    }

    /// Set calibration configuration.
    pub fn with_config(mut self, config: CalibrationConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the discount curve identifier used when pricing synthetic tranches.
    pub fn with_discount_curve_id(
        mut self,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
    ) -> Self {
        self.discount_curve_id = discount_curve_id.into();
        self
    }

    /// Set interpolation method for base correlation between detachment points.
    pub fn with_corr_interp(mut self, interp: CorrelationInterp) -> Self {
        self.corr_interp = interp;
        self
    }

    /// Set custom detachment points.
    pub fn with_detachment_points(mut self, points: Vec<f64>) -> Self {
        self.detachment_points = points;
        self
    }

    /// Set whether to use IMM dates.
    pub fn with_imm_dates(mut self, use_imm: bool) -> Self {
        self.use_imm_dates = use_imm;
        self
    }

    /// Calibrate correlation surface from tranche quotes across maturities.
    ///
    /// Returns curves indexed by maturity (in years) and a combined report
    /// with diagnostics for all calibrated maturities.
    pub fn calibrate_surface(
        &self,
        quotes: &[CreditQuote],
        market_context: &MarketContext,
    ) -> Result<(
        BTreeMap<OrderedFloat<f64>, BaseCorrelationCurve>,
        CalibrationReport,
    )> {
        // Group quotes by maturity
        let mut quotes_by_maturity: BTreeMap<OrderedFloat<f64>, Vec<&CreditQuote>> =
            BTreeMap::new();

        for quote in quotes {
            if let CreditQuote::CDSTranche { maturity, .. } = quote {
                let maturity_years = self.time_dc.year_fraction(
                    self.base_date,
                    *maturity,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                // Round to nearest target maturity
                if let Some(&target_mat) = self.target_maturities.iter().min_by(|&&a, &&b| {
                    (a - maturity_years)
                        .abs()
                        .partial_cmp(&(b - maturity_years).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    quotes_by_maturity
                        .entry(target_mat.into())
                        .or_default()
                        .push(quote);
                }
            }
        }

        let mut curves_by_maturity = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let mut total_function_evaluations = 0;
        let mut failed_maturities: Vec<String> = Vec::new();
        let mut calibrated_maturities: Vec<String> = Vec::new();

        // Calibrate each maturity separately
        for &maturity_years in &self.target_maturities {
            if let Some(maturity_quotes) = quotes_by_maturity.get(&maturity_years.into()) {
                let calibrator = BaseCorrelationCalibrator::new(
                    &self.index_id,
                    self.series,
                    maturity_years,
                    self.base_date,
                )
                .with_config(self.config.clone())
                .with_discount_curve_id(self.discount_curve_id.clone())
                .with_corr_interp(self.corr_interp)
                .with_detachment_points(self.detachment_points.clone())
                .with_imm_dates(self.use_imm_dates);

                let maturity_quote_vec: Vec<_> =
                    maturity_quotes.iter().map(|&q| q.clone()).collect();

                match calibrator.calibrate(&maturity_quote_vec, market_context) {
                    Ok((curve, report)) => {
                        curves_by_maturity.insert(maturity_years.into(), curve);
                        calibrated_maturities.push(format!("{:.1}Y", maturity_years));

                        // Merge residuals with prefixed keys
                        for (key, value) in report.residuals {
                            let prefixed_key =
                                format!("{:06}_{}Y_{}", residual_key_counter, maturity_years, key);
                            residual_key_counter += 1;
                            all_residuals.insert(prefixed_key, value);
                        }
                        total_function_evaluations += report.iterations;
                    }
                    Err(e) => {
                        // Log failure and continue with other maturities
                        tracing::warn!(
                            "Base correlation surface: failed to calibrate {:.1}Y maturity: {}",
                            maturity_years,
                            e
                        );
                        failed_maturities.push(format!("{:.1}Y", maturity_years));
                    }
                }
            } else {
                // No quotes available for this maturity
                tracing::debug!(
                    "Base correlation surface: no quotes found for {:.1}Y maturity",
                    maturity_years
                );
                failed_maturities.push(format!("{:.1}Y (no quotes)", maturity_years));
            }
        }

        // Build comprehensive report
        let solver_config = match self.config.solver_kind {
            SolverKind::Newton => SolverConfig::Newton {
                tolerance: self.config.tolerance,
                max_iterations: self.config.max_iterations,
                fd_step: 1e-8,
                min_derivative: 1e-14,
            },
            _ => SolverConfig::Brent {
                tolerance: self.config.tolerance,
                max_iterations: self.config.max_iterations,
                bracket_expansion: 1.6,
                initial_bracket_size: None,
            },
        };

        let report = CalibrationReport::for_type_with_tolerance(
            "base_correlation_surface",
            all_residuals,
            total_function_evaluations,
            self.config.tolerance,
        )
        .with_metadata("calibrated_maturities", calibrated_maturities.join(", "))
        .with_metadata("calibrated_count", curves_by_maturity.len().to_string())
        .with_metadata("time_dc", format!("{:?}", self.time_dc))
        .with_metadata("index_id", self.index_id.clone())
        .with_metadata("use_imm_dates", self.use_imm_dates.to_string())
        .with_metadata(
            "function_evaluations",
            total_function_evaluations.to_string(),
        )
        .with_metadata(
            "failed_maturities",
            if failed_maturities.is_empty() {
                "none".to_string()
            } else {
                failed_maturities.join(", ")
            },
        )
        .with_solver_config(solver_config);

        Ok((curves_by_maturity, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::CreditIndexData;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, BaseCorrelationCurve,
    };
    use std::sync::Arc;
    use time::Month;

    fn create_test_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        // Create index hazard curve
        use finstack_core::market_data::term_structures::HazardCurve;
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .expect("HazardCurve builder should succeed with valid test data");

        // Create placeholder base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![(3.0, 0.30), (10.0, 0.50)])
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index_data)
    }

    #[test]
    fn test_base_correlation_calibrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        assert_eq!(calibrator.index_id, "CDX.NA.IG.42");
        assert_eq!(calibrator.series, 42);
        assert_eq!(calibrator.maturity_years, 5.0);
        assert_eq!(
            calibrator.detachment_points,
            vec![3.0, 7.0, 10.0, 15.0, 30.0]
        );
        assert!(calibrator.use_imm_dates);
    }

    #[test]
    fn test_synthetic_tranche_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        let tranche = calibrator
            .create_synthetic_tranche(0.0, 3.0, 500.0)
            .expect("Synthetic tranche creation should succeed with valid test data");

        assert_eq!(tranche.attach_pct, 0.0);
        assert_eq!(tranche.detach_pct, 3.0);
        assert_eq!(tranche.running_coupon_bp, 500.0);
        assert_eq!(tranche.side, TrancheSide::SellProtection);
        assert!(tranche.standard_imm_dates);
    }

    #[test]
    fn test_imm_maturity_calculation() {
        // Test that IMM maturity calculation produces valid CDS roll dates
        let base_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let calibrator =
            BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date).with_imm_dates(true);

        let maturity = calibrator
            .calculate_imm_maturity()
            .expect("IMM calculation should succeed");

        // Verify maturity is a CDS IMM date (20th of Mar/Jun/Sep/Dec)
        let month = maturity.month();
        let day = maturity.day();
        assert_eq!(day, 20, "CDS IMM dates should be on the 20th");
        assert!(
            matches!(
                month,
                Month::March | Month::June | Month::September | Month::December
            ),
            "CDS IMM dates should be in Mar/Jun/Sep/Dec, got {:?}",
            month
        );
    }

    #[test]
    fn test_base_correlation_curve_building() {
        // Test direct BaseCorrelationCurve building functionality
        let correlation_knots = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let curve = BaseCorrelationCurve::builder("TEST_CORR")
            .points(correlation_knots)
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        assert_eq!(curve.detachment_points().len(), 3);
        assert_eq!(curve.correlations().len(), 3);

        // Test interpolation
        assert!((curve.correlation(5.0) - 0.35).abs() < 1e-9); // Midpoint between 3% and 7%
    }

    #[test]
    fn test_base_correlation_surface_calibrator() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let surface_calibrator = BaseCorrelationSurfaceCalibrator::new(
            "CDX.NA.IG.42",
            42,
            base_date,
            vec![3.0, 5.0, 7.0],
        );

        assert_eq!(surface_calibrator.target_maturities, vec![3.0, 5.0, 7.0]);
        assert_eq!(
            surface_calibrator.detachment_points,
            vec![3.0, 7.0, 10.0, 15.0, 30.0]
        );
        assert!(surface_calibrator.use_imm_dates);
    }

    #[test]
    fn test_initial_guess_computation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date);

        // Empty correlations should return default guess
        let guess1 = calibrator.compute_initial_guess(&[]);
        assert!((guess1 - INITIAL_CORRELATION_GUESS).abs() < 1e-9);

        // With existing correlations, should return last + step (capped)
        let solved = vec![(3.0, 0.25), (7.0, 0.45)];
        let guess2 = calibrator.compute_initial_guess(&solved);
        assert!((guess2 - (0.45 + CORRELATION_STEP)).abs() < 1e-9);

        // Near max should cap
        let solved_high = vec![(3.0, 0.88)];
        let guess3 = calibrator.compute_initial_guess(&solved_high);
        assert!(guess3 <= MAX_MONOTONIC_CORRELATION + 1e-9);
    }

    #[test]
    fn test_base_correlation_calibration_round_trip() {
        use crate::instruments::cds_tranche::pricer::CDSTranchePricer;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        // Create test market context
        let market_ctx = create_test_market_context();

        // Create known base correlation curve for validation
        let known_correlations = vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)];
        let known_curve = BaseCorrelationCurve::builder("KNOWN_CORR")
            .points(known_correlations.clone())
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        // Create market context with known correlation curve
        let original_index = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test market context");
        let test_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(&original_index.index_credit_curve))
            .base_correlation_curve(std::sync::Arc::new(known_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        let test_market_ctx = market_ctx
            .clone()
            .insert_credit_index("CDX.NA.IG.42", test_index);

        // Generate synthetic market quotes using known correlations
        let pricing_model = CDSTranchePricer::new();
        let mut synthetic_quotes = Vec::new();

        for (detach_pct, _corr) in &known_correlations {
            // Create synthetic equity tranche [0, detach_pct]
            let tranche_params = crate::instruments::cds_tranche::parameters::CDSTrancheParams::new(
                "CDX.NA.IG.42",
                42,
                0.0,         // attachment
                *detach_pct, // detachment
                Money::new(10_000_000.0, Currency::USD),
                maturity,
                500.0, // running spread
            );
            let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
            let tranche = CdsTranche::new(
                format!("EQUITY_0_{}", detach_pct),
                &tranche_params,
                &schedule_params,
                finstack_core::types::CurveId::from("USD-OIS"),
                finstack_core::types::CurveId::from("CDX.NA.IG.42"),
                TrancheSide::SellProtection,
            );

            // Price with known correlation to get "market" upfront
            let market_pv = pricing_model
                .price_tranche(&tranche, &test_market_ctx, base_date)
                .expect("Tranche pricing should succeed in test");
            let market_upfront_pct = market_pv.amount() / tranche.notional.amount() * 100.0;

            synthetic_quotes.push(CreditQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: *detach_pct,
                maturity,
                upfront_pct: market_upfront_pct,
                running_spread_bp: 500.0,
            });
        }

        // Now calibrate using these synthetic quotes
        // Use Newton solver for this smooth calibration problem (faster convergence)
        let config = CalibrationConfig {
            solver_kind: crate::calibration::SolverKind::Newton,
            tolerance: 1e-10,
            ..CalibrationConfig::default()
        };
        let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date)
            .with_config(config)
            .with_imm_dates(false); // Disable IMM for this test

        // Create clean market context for calibration (with dummy base correlation curve)
        let original_index = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test market context");

        // Create a dummy base correlation curve for initial calibration
        let dummy_base_corr_curve = BaseCorrelationCurve::builder("DUMMY_CALIB_CORR")
            .points(vec![(1.0, 0.01), (100.0, 0.01)]) // Minimal curve for building requirements
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data");

        let clean_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(&original_index.index_credit_curve))
            .base_correlation_curve(std::sync::Arc::new(dummy_base_corr_curve))
            .build()
            .expect("CreditIndexData builder should succeed with valid test data");

        let clean_market_ctx = market_ctx
            .clone()
            .insert_credit_index("CDX.NA.IG.42", clean_index);

        let calibration_result = calibrator.calibrate(&synthetic_quotes, &clean_market_ctx);

        assert!(calibration_result.is_ok());
        let (calibrated_curve, report) =
            calibration_result.expect("Calibration should succeed with synthetic quotes");

        // Verify calibration was successful
        assert!(report.success);
        assert_eq!(
            calibrated_curve.detachment_points().len(),
            known_correlations.len()
        );

        // Verify that calibrated correlations are close to known values
        for (expected_detach, expected_corr) in &known_correlations {
            let calibrated_corr = calibrated_curve.correlation(*expected_detach);
            assert!(
                (calibrated_corr - expected_corr).abs() < 0.05, // 5% tolerance for numerical precision
                "Correlation mismatch at {}%: expected {}, got {}",
                expected_detach,
                expected_corr,
                calibrated_corr
            );
        }

        // Verify calibration quality
        assert!(report.max_residual < 1e-6); // Very tight tolerance for round-trip test

        // Verify solver config is persisted
        assert!(matches!(report.solver_config, SolverConfig::Newton { .. }));

        // Verify metadata is populated
        assert!(report.metadata.contains_key("function_evaluations"));
        assert!(report.metadata.contains_key("index_id"));
        assert!(report.metadata.contains_key("validation"));
    }

    #[test]
    fn test_correlation_interp_default() {
        assert_eq!(CorrelationInterp::default(), CorrelationInterp::Linear);
    }
}

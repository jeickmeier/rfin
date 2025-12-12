//! Sequential bootstrapping algorithm for discount curve calibration.
//!
//! This module implements market-standard sequential bootstrapping where
//! discount factors are solved for one-by-one in maturity order.
//!
//! # Performance Optimization
//!
//! The solver uses a `RefCell`-based approach to avoid cloning the
//! `MarketContext` in every objective function evaluation. The curve
//! is built using `build_for_solver()` which skips non-essential validation
//! for faster iteration.

use super::DiscountCurveCalibrator;
use crate::calibration::methods::pricing::CalibrationPricer;
use crate::calibration::quote::RatesQuote;
use crate::calibration::CalibrationReport;
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::Solver;
use finstack_core::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

impl DiscountCurveCalibrator {
    /// Bootstrap discount curve from instrument quotes using solver.
    ///
    /// This method builds the curve incrementally, solving for each discount factor
    /// that reprices the corresponding instrument to par.
    ///
    /// # Algorithm
    ///
    /// 1. Sort quotes by maturity date
    /// 2. Validate quotes and curve dependencies
    /// 3. For each quote in maturity order:
    ///    - Create objective function that prices the instrument
    ///    - Use adaptive scan grid to find good starting point
    ///    - Solve for discount factor that makes PV = 0
    ///    - Add new knot to the curve
    /// 4. Build final curve with configured interpolation/extrapolation
    /// 5. Validate the calibrated curve
    ///
    /// # Features
    ///
    /// - **Adaptive scan grid**: Supports negative rate environments (DF > 1.0)
    /// - **Pre-validation**: Checks curve dependencies before bootstrap starts
    /// - **Day-count alignment**: Uses curve day count for consistent time mapping
    pub(super) fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Sort quotes by maturity
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(RatesQuote::maturity_date);

        // Validate quotes using shared validation
        let bounds = self.config.effective_rate_bounds(self.currency);
        CalibrationPricer::validate_quotes(&sorted_quotes, &bounds)?;
        CalibrationPricer::validate_discount_curve_quotes(
            &sorted_quotes,
            self.config.multi_curve.enforce_separation,
        )?;

        // Get effective curve day count for consistent time mapping
        let curve_dc = self.effective_curve_day_count();
        let pricer = self.create_pricer();

        // Pre-validate curve dependencies (fail fast for basis swaps)
        pricer.validate_curve_dependencies(&sorted_quotes, base_context)?;
        let settlement = pricer.settlement_date()?;

        // Compute spot time for explicit spot knot (if enabled)
        let t_spot = if self.include_spot_knot {
            curve_dc
                .year_fraction(
                    self.base_date,
                    settlement,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0)
        } else {
            0.0
        };

        // Build knots sequentially
        let mut knots = Vec::with_capacity(sorted_quotes.len() + 2);
        knots.push((0.0, 1.0)); // DF(0) = 1.0 at base_date

        // Add spot knot if enabled and settlement differs from base_date
        // (DF(spot) = 1.0 approximation over 0-2 business days)
        const MIN_T_SPOT: f64 = 1e-6; // ~30 seconds; avoids duplicate knots
        if self.include_spot_knot && t_spot > MIN_T_SPOT {
            knots.push((t_spot, 1.0));
        }
        let mut residuals = BTreeMap::new();
        let mut residual_key_counter: usize = 0;
        let mut total_iterations = 0;

        // Initialize explanation trace if requested
        let mut trace = if self.config.explain.enabled {
            Some(ExplanationTrace::new("calibration"))
        } else {
            None
        };

        for (idx, quote) in sorted_quotes.iter().enumerate() {
            let maturity_date = quote.maturity_date();
            // Use CURVE day count for consistent time mapping (not instrument day count)
            // This ensures all knots are on the same time basis for interpolation
            let time_to_maturity = curve_dc
                .year_fraction(
                    self.base_date,
                    maturity_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!(
                        "Year fraction calculation failed for {} maturity {}: {}",
                        self.curve_id, maturity_date, e
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                })?;

            if time_to_maturity <= 0.0 {
                continue; // Skip expired instruments
            }

            if self.config.verbose {
                tracing::debug!(
                    instrument = idx + 1,
                    total = sorted_quotes.len(),
                    maturity_date = %maturity_date,
                    time_to_maturity = time_to_maturity,
                    "Processing instrument for bootstrap"
                );
            }

            // Solve for the discount factor at this maturity
            let (solved_df, iterations) = self.solve_for_discount_factor(
                quote,
                &knots,
                time_to_maturity,
                settlement,
                curve_dc,
                &pricer,
                solver,
                base_context,
            )?;

            total_iterations += iterations;

            // Compute residual for reporting
            let final_residual = self.compute_residual(
                quote,
                &knots,
                time_to_maturity,
                solved_df,
                curve_dc,
                &pricer,
                base_context,
            )?;
            if !final_residual.is_finite()
                || final_residual.abs() >= crate::calibration::PENALTY * 0.5
            {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap pricing failed for {} at t={:.6}: residual={:?}",
                        self.curve_id, time_to_maturity, final_residual
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                });
            }

            knots.push((time_to_maturity, solved_df));

            // Store residual with descriptive key
            let key = quote.format_residual_key(residual_key_counter);
            residual_key_counter += 1;
            residuals.insert(key, final_residual);

            // Add trace entry if explanation is enabled
            if let Some(ref mut t) = trace {
                let converged = final_residual.abs() < self.config.tolerance;
                t.push(
                    TraceEntry::CalibrationIteration {
                        iteration: total_iterations,
                        residual: final_residual,
                        knots_updated: vec![format!("{:.4}y", time_to_maturity)],
                        converged,
                    },
                    self.config.explain.max_entries,
                );
            }
        }

        // Build final curve and report
        self.build_final_curve_and_report(knots, residuals, total_iterations, trace, t_spot)
    }

    /// Solve for the discount factor at a specific maturity.
    ///
    /// # Performance
    ///
    /// Uses `RefCell`-based context mutation to avoid cloning the `MarketContext`
    /// in each solver iteration. The curve is built with `build_for_solver()` which
    /// skips non-essential validation for O(1) instead of O(N^2) complexity.
    #[allow(clippy::too_many_arguments)]
    fn solve_for_discount_factor<S: Solver>(
        &self,
        quote: &RatesQuote,
        existing_knots: &[(f64, f64)],
        time_to_maturity: f64,
        settlement: finstack_core::dates::Date,
        curve_dc: finstack_core::dates::DayCount,
        pricer: &CalibrationPricer,
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(f64, usize)> {
        // Pre-allocate knots buffer with capacity for existing + candidate
        let mut temp_knots_buffer = Vec::with_capacity(existing_knots.len() + 1);
        temp_knots_buffer.extend_from_slice(existing_knots);

        // Use RefCell to allow in-place context updates without cloning
        let solver_context = Rc::new(RefCell::new(base_context.clone()));

        let quote_clone = quote.clone();
        let pricer_clone = pricer.clone();
        let base_date = self.base_date;
        let solve_interp = self.solve_interp;
        let discount_curve_id = self.effective_discount_curve_id();
        let use_ois_logic = self.use_ois_logic;

        // Capture pre-allocated buffer and RefCell context for the closure
        let temp_knots = Rc::new(RefCell::new(temp_knots_buffer));
        let ctx_rc = solver_context.clone();

        let objective = move |df: f64| -> f64 {
            // Reuse pre-allocated buffer: clear to base knots and add candidate
            let mut knots = temp_knots.borrow_mut();
            knots.truncate(existing_knots.len());
            knots.push((time_to_maturity, df));

            // Build temporary curve using fast solver path (skips full validation)
            let temp_curve = match DiscountCurve::builder(discount_curve_id.clone())
                .base_date(base_date)
                .day_count(curve_dc)
                .knots(knots.iter().copied())
                .set_interp(solve_interp)
                .allow_non_monotonic()
                .build_for_solver()
            {
                Ok(curve) => curve,
                Err(_) => return crate::calibration::PENALTY,
            };

            // Check if this instrument can be priced
            if quote_clone.requires_forward_curve()
                && (!use_ois_logic || !quote_clone.is_ois_suitable())
            {
                return crate::calibration::PENALTY;
            }

            // Update context in-place instead of cloning
            ctx_rc
                .borrow_mut()
                .insert_discount_mut(Arc::new(temp_curve));

            pricer_clone
                .price_instrument(&quote_clone, &ctx_rc.borrow())
                .unwrap_or(crate::calibration::PENALTY)
        };

        // Compute initial guess and maturity-aware DF bounds
        let initial_df = self.compute_initial_df_guess(quote, existing_knots, time_to_maturity, settlement);
        let (df_lo, df_hi) = self.df_bounds_for_time(time_to_maturity);
        let clamped_initial = initial_df.clamp(df_lo, df_hi);

        // Use maturity-aware scan grid based on rate-implied DF bounds
        let scan_grid = Self::maturity_aware_scan_grid(df_lo, df_hi, clamped_initial, 32);
        let (tentative, diag) = crate::calibration::bracket_solve_1d_with_diagnostics(
            &objective,
            clamped_initial,
            &scan_grid,
            self.config.tolerance,
            self.config.max_iterations,
        )?;

        // Track evaluations for iteration reporting
        let eval_count = diag.eval_count;

        let solved_df = if let Some(root) = tentative {
            root
        } else {
            // No bracket found - try direct solve if we have valid evaluations
            if diag.valid_eval_count == 0 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap failed for {} at t={:.6}: all {} objective evaluations returned \
                         invalid/penalized values. Scan bounds: [{:.6}, {:.6}], initial_df={:.6}. \
                         This usually indicates inconsistent conventions (calendar/settlement) or \
                         unsupported instrument type for this calibrator.",
                        self.curve_id,
                        time_to_maturity,
                        diag.eval_count,
                        diag.scan_bounds.0,
                        diag.scan_bounds.1,
                        clamped_initial
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                });
            }

            // Try direct solve from best candidate point
            let start_point = diag.best_point.unwrap_or(clamped_initial);
            solver.solve(&objective, start_point).map_err(|e| {
                finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap solver failed for {} at t={:.6}: {}. \
                         No sign-change bracket found in [{:.6}, {:.6}]. \
                         Best candidate: df={:.6} with residual={:.2e}. \
                         Evaluated {} points ({} valid).",
                        self.curve_id,
                        time_to_maturity,
                        e,
                        diag.scan_bounds.0,
                        diag.scan_bounds.1,
                        diag.best_point.unwrap_or(f64::NAN),
                        diag.best_value.unwrap_or(f64::NAN),
                        diag.eval_count,
                        diag.valid_eval_count
                    ),
                    category: "yield_curve_bootstrap".to_string(),
                }
            })?
        };

        // Validate the solution
        if !solved_df.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Bootstrap produced non-finite discount factor for {} at t={:.6}: df={:?}",
                    self.curve_id, time_to_maturity, solved_df
                ),
                category: "yield_curve_bootstrap".to_string(),
            });
        }

        // Validate against bounds
        if solved_df < df_lo || solved_df > df_hi {
            let bounds = self.config.effective_rate_bounds(self.currency);
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Solved discount factor out of bounds [{:.6}, {:.6}] implied by rate bounds [{:.4}, {:.4}] for {} at t={:.6}: df={:.6}.",
                    df_lo,
                    df_hi,
                    bounds.min_rate,
                    bounds.max_rate,
                    self.curve_id,
                    time_to_maturity,
                    solved_df
                ),
                category: "yield_curve_bootstrap".to_string(),
            });
        }

        // Return solved DF and actual evaluation count
        Ok((solved_df, eval_count))
    }

    /// Compute an initial guess for the discount factor.
    fn compute_initial_df_guess(
        &self,
        quote: &RatesQuote,
        existing_knots: &[(f64, f64)],
        time_to_maturity: f64,
        settlement: finstack_core::dates::Date,
    ) -> f64 {
        match quote {
            RatesQuote::Deposit { maturity, day_count, .. } => {
                let r = CalibrationPricer::get_rate(quote);
                let yf = day_count
                    .year_fraction(
                        settlement,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(time_to_maturity)
                    .max(1e-6);
                1.0 / (1.0 + r * yf)
            }
            _ => {
                if let Some((prev_t, prev_df)) = existing_knots.last() {
                    if time_to_maturity > *prev_t && *prev_t > 0.0 {
                        // Extrapolate forward assuming constant yield
                        let implied_rate = -prev_df.ln() / prev_t;
                        (-implied_rate * time_to_maturity).exp()
                    } else {
                        *prev_df * 0.99 // Small decay
                    }
                } else {
                    0.95 // Reasonable fallback
                }
            }
        }
    }

    /// Compute the residual (pricing error) for a solved discount factor.
    #[allow(clippy::too_many_arguments)]
    fn compute_residual(
        &self,
        quote: &RatesQuote,
        existing_knots: &[(f64, f64)],
        time_to_maturity: f64,
        solved_df: f64,
        curve_dc: finstack_core::dates::DayCount,
        pricer: &CalibrationPricer,
        base_context: &MarketContext,
    ) -> Result<f64> {
        let mut final_knots = Vec::with_capacity(existing_knots.len() + 1);
        final_knots.extend_from_slice(existing_knots);
        final_knots.push((time_to_maturity, solved_df));

        let final_curve = DiscountCurve::builder(self.effective_discount_curve_id())
            .base_date(self.base_date)
            .day_count(curve_dc)
            .knots(final_knots)
            .set_interp(self.solve_interp)
            .allow_non_monotonic()
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "temp DiscountCurve build failed for {}: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_bootstrap".to_string(),
            })?;

        if quote.requires_forward_curve()
            && (!self.use_ois_logic || !quote.is_ois_suitable())
        {
            Ok(crate::calibration::PENALTY)
        } else {
            let final_context = base_context.clone().insert_discount(final_curve);
            Ok(pricer
                .price_instrument(quote, &final_context)
                .unwrap_or(crate::calibration::PENALTY)
                .abs())
        }
    }

    /// Build the final calibrated curve and report.
    fn build_final_curve_and_report(
        &self,
        knots: Vec<(f64, f64)>,
        residuals: BTreeMap<String, f64>,
        total_iterations: usize,
        trace: Option<ExplanationTrace>,
        t_spot: f64,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Build final discount curve with configured interpolation and extrapolation
        let curve = self
            .apply_solve_interpolation(
                DiscountCurve::builder(self.curve_id.to_owned())
                    .base_date(self.base_date)
                    .day_count(self.effective_curve_day_count())
                    .extrapolation(self.extrapolation)
                    .allow_non_monotonic()
                    .knots(knots),
            )
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "final DiscountCurve build failed for {}: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_bootstrap".to_string(),
            })?;

        // Validate the calibrated curve (honor config.validation + validation_mode)
        let mut validation_status = "passed";
        let mut validation_error: Option<String> = None;
        if let Err(e) = self.validate_calibrated_curve(&curve) {
            validation_status = "failed";
            validation_error = Some(e.to_string());
            match self.config.validation_mode {
                crate::calibration::config::ValidationMode::Warn => {
                    tracing::warn!(
                        curve_id = %self.curve_id.as_str(),
                        error = %e,
                        "Calibrated discount curve failed validation (continuing due to Warn mode)"
                    );
                }
                crate::calibration::config::ValidationMode::Error => {
                    return Err(e);
                }
            }
        }

        // Create calibration report with comprehensive metadata
        let mut report = CalibrationReport::for_type_with_tolerance(
            "yield_curve",
            residuals,
            total_iterations,
            self.config.tolerance,
        )
        .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
        .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
        .with_metadata("currency", self.currency.to_string())
        .with_metadata(
            "curve_day_count",
            format!("{:?}", self.effective_curve_day_count()),
        )
        .with_metadata(
            "settlement_days",
            self.effective_settlement_days().to_string(),
        )
        .with_metadata("t_spot", format!("{:.6}", t_spot))
        .with_metadata("spot_knot_included", (t_spot > 1e-6).to_string())
        .with_metadata("allow_non_monotonic", "true")
        .with_metadata("validation", validation_status)
        .with_validation_result(validation_status == "passed", validation_error.clone());

        if let Some(err) = validation_error {
            report = report.with_metadata("validation_error", err);
        }

        // Attach explanation trace if present
        if let Some(explanation) = trace {
            report = report.with_explanation(explanation);
        }

        Ok((curve, report))
    }
}

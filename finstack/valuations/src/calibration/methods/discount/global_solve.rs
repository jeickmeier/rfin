//! Global optimization algorithm for discount curve calibration.
//!
//! This module implements Levenberg-Marquardt optimization that solves for
//! all zero rates simultaneously, minimizing the sum of squared pricing residuals.
//!
//! # Parameterization
//!
//! Optimization is performed in **zero-rate space** rather than discount factor space.
//! This provides better scaling across maturities and natural enforcement of rate bounds:
//!
//! - Parameters: `z_i` (zero rate at time `t_i`)
//! - Mapping: `DF(t_i) = exp(-z_i * t_i)`
//! - Bounds: `z_i ∈ [rate_bounds.min_rate, rate_bounds.max_rate]`

use super::DiscountCurveCalibrator;
use crate::calibration::config::RateBounds;
use crate::calibration::pricing::{CalibrationPricer, RatesQuoteUseCase};
use crate::calibration::quotes::{InstrumentConventions, RatesQuote};
use crate::calibration::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
// Note: We use solve_system_with_dim directly on LevenbergMarquardtSolver
// for safe overdetermined system support (n_residuals > n_params).
use finstack_core::prelude::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

impl DiscountCurveCalibrator {
    /// Global solve for discount factors using multi-dimensional solver.
    ///
    /// Uses Levenberg-Marquardt to minimize pricing residuals across all instruments
    /// simultaneously. This provides an optional alternative to sequential bootstrap.
    ///
    /// # Algorithm
    ///
    /// 1. Sort quotes by maturity and validate
    /// 2. Build time grid and initial guesses (using bootstrap as seed)
    /// 3. Define residual function that prices all instruments
    /// 4. Run LM optimizer to minimize sum of squared residuals
    /// 5. Build final curve from optimized discount factors
    ///
    /// # Advantages over Bootstrap
    ///
    /// - Can handle overdetermined systems (more instruments than unknowns)
    /// - Provides better convergence for ill-conditioned problems
    /// - Can incorporate regularization if needed
    ///
    /// # Disadvantages
    ///
    /// - Slower than sequential bootstrap
    /// - May not converge for very stiff problems
    /// - Less intuitive debugging (no instrument-by-instrument progress)
    pub(super) fn calibrate_global(
        &self,
        quotes: &[RatesQuote],
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        use finstack_core::error::InputError;

        // Sort quotes by maturity and validate using unified validation
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(RatesQuote::maturity_date);
        let bounds = self.config.effective_rate_bounds(self.currency);
        CalibrationPricer::validate_rates_quotes(
            &sorted_quotes,
            &bounds,
            self.base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: self.config.multi_curve.enforce_separation,
            },
        )?;

        let curve_dc = super::default_curve_day_count(self.currency);
        let pricer = self.create_pricer();
        pricer.validate_curve_dependencies(&sorted_quotes, base_context)?;
        let settlement = pricer.settlement_date(self.currency)?;

        // Compute spot knot info using the unified helper (same as bootstrap)
        let (t_spot, spot_knot) = self.compute_spot_knot(curve_dc, settlement);

        // Build time grid and initial guesses
        let (times, mut initials, active_quotes) =
            self.build_time_grid_and_guesses(&sorted_quotes, settlement, curve_dc)?;

        if active_quotes.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }

        // Seed the global solve with a high-quality initial guess from bootstrap
        self.seed_from_bootstrap(&times, &mut initials, &active_quotes, base_context)?;

        // Run the global optimization
        let (solved, residual_evals) = self.run_global_optimization(
            &times,
            &initials,
            &active_quotes,
            curve_dc,
            &pricer,
            base_context,
            spot_knot,
        )?;

        // Build final curve
        let (curve, final_knots) = self.build_global_curve(&times, &solved, curve_dc, spot_knot)?;

        // Validate calibrated curve (honor config.validation + validation_mode)
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

        // Build report with residuals (include spot knot metadata)
        let mut report = self.build_global_report(
            &active_quotes,
            &final_knots,
            curve_dc,
            &pricer,
            base_context,
            residual_evals,
            t_spot,
        )?;

        report = report
            .with_metadata("validation", validation_status)
            .with_validation_result(validation_status == "passed", validation_error.clone());
        if let Some(err) = validation_error {
            report = report.with_metadata("validation_error", err);
        }

        Ok((curve, report))
    }

    /// Build the time grid and initial zero-rate guesses from sorted quotes.
    ///
    /// Returns (times, initial_zero_rates, active_quotes).
    fn build_time_grid_and_guesses(
        &self,
        sorted_quotes: &[RatesQuote],
        settlement: finstack_core::dates::Date,
        curve_dc: finstack_core::dates::DayCount,
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<RatesQuote>)> {
        let mut times: Vec<f64> = Vec::new();
        let mut initials: Vec<f64> = Vec::new();
        let mut active_quotes: Vec<RatesQuote> = Vec::new();
        let bounds = self.config.effective_rate_bounds(self.currency);

        for quote in sorted_quotes {
            let maturity_date = quote.maturity_date();
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
                    category: "yield_curve_global_solve".to_string(),
                })?;

            if time_to_maturity <= 0.0 {
                continue;
            }

            // Compute initial zero rate guess from the quote
            let init_zero_rate = match quote {
                RatesQuote::Deposit { maturity, .. } => {
                    // Deposits are quoted on the accrual from *settlement* → maturity
                    let r = CalibrationPricer::get_rate(quote);
                    let day_count = quote
                        .conventions()
                        .day_count
                        .unwrap_or_else(|| InstrumentConventions::default_money_market_day_count(self.currency));
                    let yf = day_count
                        .year_fraction(
                            settlement,
                            *maturity,
                            finstack_core::dates::DayCountCtx::default(),
                        )
                        .unwrap_or(time_to_maturity)
                        .max(1e-6);
                    let df = 1.0 / (1.0 + r * yf);
                    // Convert DF to zero rate: z = -ln(df) / t
                    Self::df_to_zero_rate(df, time_to_maturity)
                }
                _ => {
                    // For swaps/FRAs, the quoted rate is a good approximation
                    CalibrationPricer::get_rate(quote)
                }
            };

            times.push(time_to_maturity);
            initials.push(init_zero_rate.clamp(bounds.min_rate, bounds.max_rate));
            active_quotes.push(quote.clone());
        }

        Ok((times, initials, active_quotes))
    }

    /// Convert discount factor to zero rate: z = -ln(df) / t
    #[inline]
    fn df_to_zero_rate(df: f64, t: f64) -> f64 {
        if t <= 1e-12 || df <= 0.0 {
            return 0.0;
        }
        -df.ln() / t
    }

    /// Convert zero rate to discount factor: df = exp(-z * t)
    #[inline]
    fn zero_rate_to_df(z: f64, t: f64) -> f64 {
        (-z * t).exp()
    }

    /// Seed initial zero-rate guesses from bootstrap solution.
    fn seed_from_bootstrap(
        &self,
        times: &[f64],
        initials: &mut [f64],
        active_quotes: &[RatesQuote],
        base_context: &MarketContext,
    ) -> Result<()> {
        let bounds = self.config.effective_rate_bounds(self.currency);
        if let Ok((boot_curve, _)) =
            self.bootstrap_curve(active_quotes, base_context)
        {
            for (i, t) in times.iter().enumerate() {
                let df = boot_curve.df(*t);
                let z = Self::df_to_zero_rate(df, *t);
                initials[i] = z.clamp(bounds.min_rate, bounds.max_rate);
            }
        }
        Ok(())
    }

    /// Run the Levenberg-Marquardt optimization in zero-rate space.
    ///
    /// Returns the solved zero-rate parameters and the number of residual evaluations.
    #[allow(clippy::too_many_arguments)]
    fn run_global_optimization(
        &self,
        times: &[f64],
        initials: &[f64],
        active_quotes: &[RatesQuote],
        curve_dc: finstack_core::dates::DayCount,
        pricer: &CalibrationPricer,
        base_context: &MarketContext,
        spot_knot: Option<(f64, f64)>,
    ) -> Result<(Vec<f64>, usize)> {
        let solver = self.config.create_lm_solver();
        let base_context_clone = base_context.clone();
        let solve_interp = self.solve_interp;
        let base_date = self.base_date;
        let discount_curve_id = self.effective_discount_curve_id();
        let pricer_clone = pricer.clone();
        let times_clone = times.to_vec();
        let active_quotes_clone = active_quotes.to_vec();
        let n_residuals = active_quotes.len();
        let rate_bounds = self.config.effective_rate_bounds(self.currency);
        let currency = self.currency;

        // Track residual evaluation count
        let eval_counter = Arc::new(AtomicUsize::new(0));
        let eval_counter_clone = Arc::clone(&eval_counter);

        let residuals = move |params: &[f64], resid: &mut [f64]| {
            eval_counter_clone.fetch_add(1, Ordering::Relaxed);

            // DETERMINISM: Always overwrite the entire residual buffer first.
            // This ensures no stale values leak from previous evaluations, even if
            // the solver provides a buffer larger than n_residuals.
            for r in resid.iter_mut() {
                *r = 0.0;
            }

            // Safety guard: only write within bounds
            debug_assert!(
                resid.len() >= n_residuals,
                "Residual buffer too small: {} < {}",
                resid.len(),
                n_residuals
            );

            // Build curve once per parameter vector (params are zero rates)
            let knots =
                Self::build_knots_from_zero_rates(&times_clone, params, &rate_bounds, spot_knot);

            // Use build_for_solver() for fast path (skips non-essential validation)
            let temp_curve = match DiscountCurve::builder(discount_curve_id.clone())
                .base_date(base_date)
                .day_count(curve_dc)
                .knots(knots)
                .set_interp(solve_interp)
                .allow_non_monotonic()
                .build_for_solver()
            {
                Ok(curve) => curve,
                Err(_) => {
                    // Fill residuals with penalty (buffer already zeroed above)
                    for r in resid.iter_mut().take(n_residuals) {
                        *r = crate::calibration::PENALTY;
                    }
                    return;
                }
            };

            let temp_context = base_context_clone.clone().insert_discount(temp_curve);

            for (i, quote) in active_quotes_clone.iter().enumerate().take(n_residuals) {
                resid[i] = pricer_clone
                    .price_instrument(quote, currency, &temp_context)
                    .unwrap_or(crate::calibration::PENALTY);
            }
        };

        // Use solve_system_with_dim for safe overdetermined support
        let result = solver.solve_system_with_dim(residuals, initials, n_residuals)?;
        let evals = eval_counter.load(Ordering::Relaxed);
        Ok((result, evals))
    }

    /// Build knots from zero-rate parameters, converting to DFs.
    ///
    /// Applies rate bounds clamping and hard numeric guards but does NOT enforce
    /// DF-direction monotonicity. This supports negative-rate regimes where
    /// DFs may increase with maturity (DF > 1 for negative rates).
    ///
    /// # Arguments
    /// * `times` - Time points for each knot
    /// * `zero_rates` - Zero rate parameters (z_i)
    /// * `rate_bounds` - Rate bounds for clamping
    /// * `spot_knot` - Optional spot knot to include after the base knot
    pub(crate) fn build_knots_from_zero_rates(
        times: &[f64],
        zero_rates: &[f64],
        rate_bounds: &RateBounds,
        spot_knot: Option<(f64, f64)>,
    ) -> Vec<(f64, f64)> {
        let mut knots = Vec::with_capacity(zero_rates.len() + 2);
        knots.push((0.0, 1.0));

        // Add spot knot if provided (for OIS spot anchoring)
        if let Some(knot) = spot_knot {
            knots.push(knot);
        }

        for (&t, &raw_z) in times.iter().zip(zero_rates.iter()) {
            // Clamp zero rate to configured bounds
            let z = raw_z.clamp(rate_bounds.min_rate, rate_bounds.max_rate);

            // Convert zero rate to discount factor: df = exp(-z * t)
            let mut df = Self::zero_rate_to_df(z, t);

            // Hard numerical guard to avoid degenerate DFs (NaN, 0, or extreme values)
            const DF_HARD_MIN: f64 = 1e-12;
            const DF_HARD_MAX: f64 = 1e6;
            df = df.clamp(DF_HARD_MIN, DF_HARD_MAX);

            // Note: No DF-direction enforcement here. Negative-rate regimes may have
            // DF(t) > 1.0 or increasing DFs. The curve builder and validator handle
            // any interpolator-specific constraints.
            knots.push((t, df));
        }

        knots
    }

    /// Build the final curve from solved zero-rate parameters.
    fn build_global_curve(
        &self,
        times: &[f64],
        solved_zero_rates: &[f64],
        curve_dc: finstack_core::dates::DayCount,
        spot_knot: Option<(f64, f64)>,
    ) -> Result<(DiscountCurve, Vec<(f64, f64)>)> {
        let bounds = self.config.effective_rate_bounds(self.currency);
        let final_knots =
            Self::build_knots_from_zero_rates(times, solved_zero_rates, &bounds, spot_knot);

        // Build final curve using the unified pathway (no solver-only flags)
        let curve = self.build_curve(self.curve_id.to_owned(), curve_dc, final_knots.clone())?;

        Ok((curve, final_knots))
    }

    /// Build the calibration report with residuals.
    #[allow(clippy::too_many_arguments)]
    fn build_global_report(
        &self,
        active_quotes: &[RatesQuote],
        final_knots: &[(f64, f64)],
        curve_dc: finstack_core::dates::DayCount,
        pricer: &CalibrationPricer,
        base_context: &MarketContext,
        residual_evals: usize,
        t_spot: f64,
    ) -> Result<CalibrationReport> {
        let mut residuals_map = BTreeMap::new();
        let mut residual_values = Vec::with_capacity(active_quotes.len());

        // Build an equivalent curve under the effective discount curve ID for residual reporting
        let pricing_curve = self.build_curve(
            self.effective_discount_curve_id(),
            curve_dc,
            final_knots.to_vec(),
        )?;

        let final_context = base_context.clone().insert_discount(pricing_curve);
        for (idx, quote) in active_quotes.iter().enumerate() {
            let ctx = final_context.clone();
            let residual = pricer
                .price_instrument(quote, self.currency, &ctx)
                .unwrap_or(crate::calibration::PENALTY)
                .abs();
            residuals_map.insert(format!("GLOBAL-{:06}", idx), residual);
            residual_values.push(residual);
        }

        // Compute final residual metrics
        let l2_norm: f64 = residual_values.iter().map(|r| r * r).sum::<f64>().sqrt();
        let max_abs_residual = residual_values.iter().copied().fold(0.0_f64, f64::max);

        let report = CalibrationReport::for_type_with_tolerance(
            "yield_curve_global",
            residuals_map,
            residual_evals,
            self.config.tolerance,
        )
        .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
        .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
        .with_metadata("currency", self.currency.to_string())
        .with_metadata(
            "curve_day_count",
            format!("{:?}", super::default_curve_day_count(self.currency)),
        )
        /* settlement_days metadata removed */
        .with_metadata("t_spot", format!("{:.6}", t_spot))
        .with_metadata("spot_knot_included", self.include_spot_knot.to_string())
        .with_metadata("method", "global_solve")
        .with_metadata("residual_evals", residual_evals.to_string())
        .with_metadata("l2_norm", format!("{:.2e}", l2_norm))
        .with_metadata("max_abs_residual", format!("{:.2e}", max_abs_residual));

        Ok(report)
    }
}

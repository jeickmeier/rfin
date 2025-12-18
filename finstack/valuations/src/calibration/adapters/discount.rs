//! Discount curve calibration adapter.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::config::ResidualWeightingScheme;
use crate::calibration::constants::*;
use crate::calibration::pricing::CalibrationPricer;
use crate::calibration::pricing::convention_resolution as conv;
use crate::calibration::quotes::RatesQuote;
use crate::calibration::solver::{BootstrapTarget, GlobalSolveTarget};
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};
use std::cell::RefCell;

/// Parameters for constructing a `DiscountCurveTarget`.
#[derive(Clone)]
pub struct DiscountCurveTargetParams {
    /// Base date for the curve.
    pub base_date: Date,
    /// Currency of the curve.
    pub currency: Currency,
    /// Identifier for the curve being built.
    pub curve_id: CurveId,
    /// Effective ID for pricing (usually same as curve_id).
    pub discount_curve_id: CurveId,
    /// Effective ID for pricing forward rates.
    pub forward_curve_id: CurveId,
    /// Interpolation style for solving.
    pub solve_interp: InterpStyle,
    /// Extrapolation policy.
    pub extrapolation: ExtrapolationPolicy,
    /// Calibration configuration.
    pub config: CalibrationConfig,
    /// Pricer for calibration instruments.
    pub pricer: CalibrationPricer,
    /// Day count convention for the curve.
    pub curve_day_count: DayCount,
    /// Optional spot knot (t_spot, 1.0) if enabled.
    pub spot_knot: Option<(f64, f64)>,
    /// Settlement date.
    pub settlement_date: Date,
    /// Context needed for pricing against OTHER curves (if any).
    pub base_context: MarketContext,
}

/// Target for discount curve calibration (Bootstrap and Global).
pub struct DiscountCurveTarget {
    /// Base date for the curve.
    pub base_date: Date,
    /// Currency of the curve.
    pub currency: Currency,
    /// Identifier for the curve being built.
    pub curve_id: CurveId,
    /// Effective ID for pricing (usually same as curve_id).
    pub discount_curve_id: CurveId,
    /// Effective ID for pricing forward rates.
    pub forward_curve_id: CurveId,
    /// Interpolation style for solving.
    pub solve_interp: InterpStyle,
    /// Extrapolation policy.
    pub extrapolation: ExtrapolationPolicy,
    /// Calibration configuration.
    pub config: CalibrationConfig,
    /// Pricer for calibration instruments.
    pub pricer: CalibrationPricer,
    /// Day count convention for the curve.
    pub curve_day_count: DayCount,
    /// Optional spot knot (t_spot, 1.0) if enabled.
    pub spot_knot: Option<(f64, f64)>,
    /// Settlement date.
    pub settlement_date: Date,
    /// Context needed for pricing against OTHER curves (if any).
    pub base_context: MarketContext,
    /// Optional reusable context for sequential solvers.
    reuse_context: Option<RefCell<MarketContext>>,
}

impl DiscountCurveTarget {
    /// Create a new `DiscountCurveTarget` from parameters.
    pub fn new(params: DiscountCurveTargetParams) -> Self {
        let reuse_context = if params.config.use_parallel {
            None
        } else {
            Some(RefCell::new(params.base_context.clone()))
        };
        Self {
            base_date: params.base_date,
            currency: params.currency,
            curve_id: params.curve_id,
            discount_curve_id: params.discount_curve_id,
            forward_curve_id: params.forward_curve_id,
            solve_interp: params.solve_interp,
            extrapolation: params.extrapolation,
            config: params.config,
            pricer: params.pricer,
            curve_day_count: params.curve_day_count,
            settlement_date: params.settlement_date,
            spot_knot: params.spot_knot,
            base_context: params.base_context,
            reuse_context,
        }
    }

    /// Compute DF bounds for time t based on rate bounds.
    fn df_bounds_for_time(&self, time: f64) -> (f64, f64) {
        let bounds = self.config.effective_rate_bounds(self.currency);
        let df_lo = (-bounds.max_rate * time).exp();
        let df_hi = (-bounds.min_rate * time).exp();
        (df_lo, df_hi)
    }

    /// Helper to convert zero rate to DF.
    fn zero_rate_to_df(z: f64, t: f64) -> f64 {
        (-z * t).exp()
    }

    /// Generate maturity-aware scan grid.
    fn maturity_aware_scan_grid(
        df_lo: f64,
        df_hi: f64,
        initial_df: f64,
        num_points: usize,
    ) -> Vec<f64> {
        let mut grid = Vec::with_capacity(num_points + 20);

        if df_lo.is_finite() && df_lo > 0.0 {
            grid.push(df_lo);
        }
        if df_hi.is_finite() && df_hi > 0.0 && (df_hi - df_lo).abs() > TOLERANCE_DUP_KNOTS {
            grid.push(df_hi);
        }

        let center = initial_df.clamp(df_lo, df_hi);
        let use_log_spacing = df_lo > DF_MIN_HARD && df_hi / df_lo > 10.0;

        if use_log_spacing {
            let log_lo = df_lo.ln();
            let log_hi = df_hi.ln();
            let coarse_points = num_points.saturating_sub(10).max(8);
            for i in 1..coarse_points {
                let t = i as f64 / coarse_points as f64;
                let log_df = log_lo + t * (log_hi - log_lo);
                let df = log_df.exp();
                if df.is_finite() && df > 0.0 && df > df_lo && df < df_hi {
                    grid.push(df);
                }
            }
        } else {
            let coarse_points = num_points.saturating_sub(10).max(8);
            for i in 1..coarse_points {
                let t = i as f64 / coarse_points as f64;
                let df = df_lo + t * (df_hi - df_lo);
                if df.is_finite() && df > 0.0 && df > df_lo && df < df_hi {
                    grid.push(df);
                }
            }
        }

        if center.is_finite() && center > 0.0 {
            grid.push(center);
            let log_center = center.ln();
            let log_lo = df_lo.max(DF_MIN_HARD).ln();
            let log_hi = df_hi.max(DF_MIN_HARD).ln();
            const LOG_STEPS: [f64; 8] = [1e-4, 2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2];
            for step in LOG_STEPS {
                for sign in [-1.0, 1.0] {
                    let candidate = log_center + sign * step;
                    if candidate >= log_lo && candidate <= log_hi {
                        let df = candidate.exp();
                        if df >= df_lo && df <= df_hi && df.is_finite() && df > 0.0 {
                            grid.push(df);
                        }
                    }
                }
            }
        }

        grid.sort_by(|a, b| b.total_cmp(a));
        grid.dedup_by(|a, b| (*a - *b).abs() < (df_hi - df_lo) * TOLERANCE_GRID_DEDUP);
        grid
    }

    fn with_temp_context<F, T>(&self, curve: &DiscountCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();
            ctx.insert_mut(curve.clone());
            op(&ctx)
        } else {
            let temp_context = self.base_context.clone().insert_discount(curve.clone());
            op(&temp_context)
        }
    }

    fn knots_from_params(&self, times: &[f64], params: &[f64]) -> Result<Vec<(f64, f64)>> {
        if times.len() != params.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve dimension mismatch: {} times vs {} params",
                    times.len(),
                    params.len()
                ),
                category: "global_solve".to_string(),
            });
        }

        let mut knots = Vec::with_capacity(times.len() + 2);
        knots.push((0.0, 1.0));

        if let Some(spot) = self.spot_knot {
            if spot.0 > 0.0 {
                knots.push(spot);
            }
        }

        let bounds = self.config.effective_rate_bounds(self.currency);
        let mut last_t = self.spot_knot.map(|spot| spot.0).unwrap_or(0.0);

        for (&t, &z) in times.iter().zip(params.iter()) {
            if t <= last_t {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Non-increasing knot time {:.10} detected (previous {:.10}). \
Global solve requires strictly increasing times.",
                        t, last_t
                    ),
                    category: "global_solve".to_string(),
                });
            }
            last_t = t;
            let clamped_z = z.clamp(bounds.min_rate, bounds.max_rate);
            let mut df = Self::zero_rate_to_df(clamped_z, t);
            df = df.clamp(
                self.config.discount_curve.df_hard_min,
                self.config.discount_curve.df_hard_max,
            );
            knots.push((t, df));
        }

        Ok(knots)
    }
}

impl BootstrapTarget for DiscountCurveTarget {
    type Quote = RatesQuote;
    type Curve = DiscountCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let pillar_date = match quote {
            RatesQuote::Swap { maturity, .. } => {
                // Align the calibration knot with the actual swap payment date (maturity plus
                // payment delay in business days). This avoids extrapolating over the small
                // interval [maturity, maturity+lag] when OIS conventions apply a payment lag.
                let resolved = conv::resolve_swap_conventions(&self.pricer, quote, self.currency)?;
                crate::instruments::irs::dates::add_payment_delay(
                    *maturity,
                    resolved.common.payment_delay_days,
                    Some(resolved.common.payment_calendar_id),
                )
            }
            _ => quote.maturity_date(),
        };
        self.curve_day_count
            .year_fraction(
                self.base_date,
                pillar_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("YF calc failed: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.build_curve_for_solver(knots)
    }

    fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        if knots.len() < 2 {
            return Err(finstack_core::Error::Calibration {
                message: "Failed to build temp curve: need at least two knots".into(),
                category: "bootstrapping".to_string(),
            });
        }
        if knots.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Failed to build temp curve: non-increasing knot times: {:?}",
                    knots
                        .windows(2)
                        .find(|w| w[1].0 <= w[0].0)
                        .map(|w| (w[0].0, w[1].0))
                ),
                category: "bootstrapping".to_string(),
            });
        }

        DiscountCurve::builder(self.discount_curve_id.clone())
            .base_date(self.base_date)
            .day_count(self.curve_day_count)
            .knots(knots.iter().copied())
            .set_interp(self.solve_interp)
            .extrapolation(self.extrapolation)
            .allow_non_monotonic()
            .build_for_solver()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Failed to build temp curve: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        let config_flag = self.config.discount_curve.allow_non_monotonic_final;
        let policy_allow = match self.config.rate_bounds_policy {
            crate::calibration::RateBoundsPolicy::Explicit => {
                self.config.rate_bounds.min_rate < 0.0
            }
            crate::calibration::RateBoundsPolicy::AutoCurrency => {
                matches!(self.currency, Currency::EUR | Currency::JPY | Currency::CHF)
            }
        };
        let allow_non_monotonic = config_flag.unwrap_or(policy_allow);

        if allow_non_monotonic && self.solve_interp == InterpStyle::MonotoneConvex {
            return Err(finstack_core::Error::Calibration {
                message: "MonotoneConvex interpolation requires non-increasing discount factors. \
Disable allow_non_monotonic_final or choose a compatible interpolation style."
                    .to_string(),
                category: "curve_build".to_string(),
            });
        }

        let mut builder = DiscountCurve::builder(self.curve_id.clone())
            .base_date(self.base_date)
            .day_count(self.curve_day_count)
            .knots(knots.iter().copied())
            .set_interp(self.solve_interp)
            .extrapolation(self.extrapolation);

        if allow_non_monotonic {
            builder = builder.allow_non_monotonic();
        } else {
            builder = builder.enforce_no_arbitrage();
        }

        builder
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: e.to_string(),
                category: "curve_build".to_string(),
            })
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        self.with_temp_context(curve, |ctx| {
            self.pricer
                .price_instrument_for_calibration(quote, self.currency, ctx)
        })
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let t = self.quote_time(quote)?;
        let (df_lo, df_hi) = self.df_bounds_for_time(t);

        match quote {
            RatesQuote::Deposit { maturity, .. } => {
                let r = CalibrationPricer::get_rate(quote);
                let day_count = quote.conventions().day_count.ok_or_else(|| {
                    finstack_core::Error::Validation(
                        "Deposit quote requires conventions.day_count to be set".to_string(),
                    )
                })?;
                let settlement_start = if quote.conventions().settlement_days.is_some()
                    || quote.conventions().calendar_id.is_some()
                    || quote.conventions().business_day_convention.is_some()
                {
                    self.pricer
                        .settlement_date_for_quote(quote.conventions(), self.currency)?
                } else {
                    self.settlement_date
                };
                let yf = day_count
                    .year_fraction(
                        settlement_start,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )?
                    .max(1e-6);
                let df: f64 = 1.0 / (1.0 + r * yf);
                Ok(df.clamp(df_lo, df_hi))
            }
            _ => {
                let mut guess = None;
                let last_two: Vec<(f64, f64)> = previous_knots
                    .iter()
                    .copied()
                    .rev()
                    .filter(|(ti, dfi)| *ti > MIN_GRID_SPACING && dfi.is_finite() && *dfi > 0.0)
                    .take(2)
                    .collect();

                if last_two.len() == 2 {
                    let (t1, df1) = last_two[0];
                    let (t0, df0) = last_two[1];
                    let dt = t1 - t0;
                    if dt > MIN_GRID_SPACING {
                        let f = (df0.ln() - df1.ln()) / dt;
                        let ln_df = df1.ln() - f * (t - t1);
                        let df = ln_df.exp();
                        if df.is_finite() && df > 0.0 {
                            guess = Some(df);
                        }
                    }
                } else if last_two.len() == 1 {
                    let (t1, df1) = last_two[0];
                    if t1 > MIN_GRID_SPACING {
                        let df = df1.powf(t / t1);
                        if df.is_finite() && df > 0.0 {
                            guess = Some(df);
                        }
                    }
                }

                let df = guess.ok_or_else(|| finstack_core::Error::Calibration {
                    message: "Unable to derive initial DF guess".into(),
                    category: "bootstrapping".to_string(),
                })?;
                Ok(df.clamp(df_lo, df_hi))
            }
        }
    }

    fn scan_points(&self, quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        let time = self.quote_time(quote)?;
        let (df_lo, df_hi) = self.df_bounds_for_time(time);
        let clamped_initial = initial_guess.clamp(df_lo, df_hi);

        let num_points = self.config.discount_curve.scan_grid_points;
        let min_points = self.config.discount_curve.min_scan_grid_points;
        let mut grid = Self::maturity_aware_scan_grid(df_lo, df_hi, clamped_initial, num_points);

        if grid.len() < min_points {
            let num_additional = min_points - grid.len().min(min_points);
            for i in 0..=num_additional {
                let t = i as f64 / num_additional as f64;
                let df = df_lo + t * (df_hi - df_lo);
                if df.is_finite() && df > 0.0 {
                    grid.push(df);
                }
            }
            grid.sort_by(|a, b| b.total_cmp(a));
            grid.dedup_by(|a, b| (*a - *b).abs() < (df_hi - df_lo) * TOLERANCE_GRID_DEDUP);
        }

        Ok(grid)
    }

    fn validate_knot(&self, time: f64, value: f64) -> Result<()> {
        if !value.is_finite() || value <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Non-finite or non-positive discount factor at t={:.6}",
                    time
                ),
                category: "bootstrapping".to_string(),
            });
        }
        let (lo, hi) = self.df_bounds_for_time(time);
        if value < lo || value > hi {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "DF out of bounds at t={:.4}: got {:.6}, range [{:.6}, {:.6}]",
                    time, value, lo, hi
                ),
                category: "bootstrapping".to_string(),
            });
        }
        Ok(())
    }
}

impl GlobalSolveTarget for DiscountCurveTarget {
    type Quote = RatesQuote;
    type Curve = DiscountCurve;

    fn build_time_grid_and_guesses(
        &self,
        quotes: &[Self::Quote],
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<Self::Quote>)> {
        let bounds = self.config.effective_rate_bounds(self.currency);
        let mut entries = Vec::new();

        for quote in quotes {
            let t = self.quote_time(quote)?;
            if t <= 0.0 {
                continue;
            }

            // Initial guess (zero rate)
            // Simplified: use quote rate.
            // Note: For Global Solve, quotes are often swaps where rate ~ zero rate for flat curves.
            let rate = CalibrationPricer::get_rate(quote);
            let z = rate.clamp(bounds.min_rate, bounds.max_rate);
            entries.push((t, z, quote.clone()));
        }

        entries.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut times = Vec::with_capacity(entries.len());
        let mut initials = Vec::with_capacity(entries.len());
        let mut active_quotes = Vec::with_capacity(entries.len());
        let mut last_time: Option<f64> = None;

        for (t, z, quote) in entries {
            if let Some(prev) = last_time {
                if (t - prev).abs() <= TOLERANCE_DUP_KNOTS {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Duplicate or unsorted knot times detected (prev={:.10}, new={:.10}). \
Ensure quotes map to strictly increasing year fractions.",
                            prev, t
                        ),
                        category: "global_solve".to_string(),
                    });
                }
            }
            last_time = Some(t);
            times.push(t);
            initials.push(z);
            active_quotes.push(quote);
        }

        Ok((times, initials, active_quotes))
    }

    fn build_curve_from_params(&self, times: &[f64], params: &[f64]) -> Result<Self::Curve> {
        self.build_curve_for_solver_from_params(times, params)
    }

    fn build_curve_for_solver_from_params(
        &self,
        times: &[f64],
        params: &[f64],
    ) -> Result<Self::Curve> {
        let knots = self.knots_from_params(times, params)?;
        self.build_curve_for_solver(&knots)
    }

    fn build_curve_final_from_params(&self, times: &[f64], params: &[f64]) -> Result<Self::Curve> {
        let knots = self.knots_from_params(times, params)?;
        self.build_curve_final(&knots)
    }

    fn calculate_residuals(
        &self,
        curve: &Self::Curve,
        quotes: &[Self::Quote],
        residuals: &mut [f64],
    ) -> Result<()> {
        self.with_temp_context(curve, |ctx| {
            for (i, quote) in quotes.iter().enumerate() {
                if i >= residuals.len() {
                    break;
                }
                residuals[i] =
                    self.pricer
                        .price_instrument_for_calibration(quote, self.currency, ctx)?;
            }
            Ok(())
        })
    }

    fn residual_key(&self, quote: &Self::Quote, idx: usize) -> String {
        use RatesQuote::*;
        let prefix = match quote {
            Deposit { .. } => "DEP",
            FRA { .. } => "FRA",
            Future { .. } => "FUT",
            Swap { .. } => {
                if quote.is_ois_suitable() {
                    "OIS"
                } else {
                    "SWP"
                }
            }
            BasisSwap { .. } => "BAS",
        };
        let maturity = quote.maturity_date();
        format!("{}-{}-{:03}", prefix, maturity, idx)
    }

    fn residual_weights(&self, quotes: &[Self::Quote], weights_out: &mut [f64]) -> Result<()> {
        if quotes.len() != weights_out.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve requires weights.len() == quotes.len(); got {} vs {}.",
                    weights_out.len(),
                    quotes.len()
                ),
                category: "global_solve".to_string(),
            });
        }

        for (i, quote) in quotes.iter().enumerate() {
            let t = self.quote_time(quote)?.max(1e-6);

            let weight = match self.config.discount_curve.weighting_scheme {
                ResidualWeightingScheme::Equal => 1.0,
                ResidualWeightingScheme::LinearTime => t,
                ResidualWeightingScheme::SqrtTime => t.sqrt(),
                ResidualWeightingScheme::InverseDuration => {
                    // Approximation: Duration ~ t for zero-coupon, ~ fixed_leg_duration for swaps
                    // For calibration, quotes are par instruments, so residuals are usually in Rate space.
                    // Sensitivity of Par Rate to curve bumps (DV01) is proportional to duration.
                    // We weight by 1/DV01 to normalize to "Price-like" errors or equalize importance.
                    // Simplified: duration ~ t.
                    1.0 / t.max(0.1)
                }
            };

            weights_out[i] = weight.max(WEIGHT_MIN_FLOOR);
        }
        Ok(())
    }

    fn jacobian(
        &self,
        params: &[f64],
        times: &[f64],
        quotes: &[Self::Quote],
        jacobian: &mut [Vec<f64>],
    ) -> Result<()> {
        // Validation of dimensions
        if jacobian.len() != quotes.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Jacobian rows {} != quotes {}",
                    jacobian.len(),
                    quotes.len()
                ),
                category: "jacobian".to_string(),
            });
        }
        if params.len() != times.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!("Params {} != Times {}", params.len(), times.len()),
                category: "jacobian".to_string(),
            });
        }
        // Check columns
        if !jacobian.is_empty() && jacobian[0].len() != params.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Jacobian cols {} != params {}",
                    jacobian[0].len(),
                    params.len()
                ),
                category: "jacobian".to_string(),
            });
        }

        // 1. Calculate base residuals
        let base_curve = self.build_curve_for_solver_from_params(times, params)?;
        let mut base_residuals = vec![0.0; quotes.len()];
        self.calculate_residuals(&base_curve, quotes, &mut base_residuals)?;

        // Use configured step size
        let fd_eps = self.config.discount_curve.jacobian_step_size;
        let mut params_plus = params.to_vec();

        // Optimization: Use a single mutable context to avoid cloning base_context in every iteration.
        // We clone once, then update the discount curve in-place (or by move-and-update).
        let mut temp_context = self.base_context.clone();

        // 2. Iterate parameters (columns)
        for j in 0..params.len() {
            let _t_knot = times[j];
            let p_orig = params[j];

            // Bump parameter
            let h = (p_orig.abs() * fd_eps).max(fd_eps);
            params_plus[j] = p_orig + h;

            // Build bumped curve
            let curve_plus = self.build_curve_for_solver_from_params(times, &params_plus)?;

            // Update context with bumped curve
            // optimizing: insert_discount consumes self and returns Self, effectively managing internal state efficiently
            temp_context = temp_context.insert_discount(curve_plus);

            // 3. Iterate quotes (rows)
            // Optimization: Only re-price quotes that *could* depend on t_knot.
            let t_cutoff = if j > 0 { times[j - 1] } else { 0.0 };

            let ctx = &temp_context;
            for (i, quote) in quotes.iter().enumerate() {
                // Skip if quote is "safely" before the bumped knot
                let t_quote = self.quote_time(quote)?;

                if t_quote < t_cutoff - 1e-4 {
                    // Derivative is effectively 0
                    jacobian[i][j] = 0.0;
                    continue;
                }

                let val_plus =
                    self.pricer
                        .price_instrument_for_calibration(quote, self.currency, ctx)?;
                let val_base = base_residuals[i];

                jacobian[i][j] = (val_plus - val_base) / h;
            }

            // Reset parameter
            params_plus[j] = p_orig;
        }

        Ok(())
    }

    fn supports_analytical_jacobian(&self) -> bool {
        // We now support a specialized efficient Jacobian calculation
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::pricing::CalibrationPricer;
    use crate::calibration::quotes::conventions::InstrumentConventions;
    use crate::calibration::solver::BootstrapTarget;
    use crate::calibration::solver::GlobalFitOptimizer;
    use finstack_core::dates::DayCountCtx;
    use finstack_core::dates::Tenor;
    use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
    use finstack_core::types::CurveId;
    use time::Month;

    #[test]
    fn discount_quote_time_uses_swap_payment_date_when_payment_delay_applies() {
        let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("base_date");
        let maturity = Date::from_calendar_date(2024, Month::January, 4).expect("maturity");

        let pricer =
            CalibrationPricer::new(base_date, "USD-OIS").with_market_conventions(Currency::USD);

        let quote = RatesQuote::Swap {
            maturity,
            rate: 0.02,
            is_ois: false,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default(),
            float_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-OIS"),
        };

        let resolved = conv::resolve_swap_conventions(&pricer, &quote, Currency::USD)
            .expect("resolved");
        assert_eq!(resolved.common.payment_delay_days, 2);

        let pay_date = crate::instruments::irs::dates::add_payment_delay(
            maturity,
            resolved.common.payment_delay_days,
            Some(resolved.common.payment_calendar_id),
        );
        let expected = DayCount::Act365F
            .year_fraction(base_date, pay_date, DayCountCtx::default())
            .expect("yf");

        let target = DiscountCurveTarget::new(DiscountCurveTargetParams {
            base_date,
            currency: Currency::USD,
            curve_id: CurveId::new("USD-OIS"),
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-OIS"),
            solve_interp: InterpStyle::Linear,
            extrapolation: ExtrapolationPolicy::FlatZero,
            config: CalibrationConfig::default(),
            pricer,
            curve_day_count: DayCount::Act365F,
            spot_knot: None,
            settlement_date: base_date,
            base_context: MarketContext::new(),
        });

        let actual = BootstrapTarget::quote_time(&target, &quote).expect("quote_time");
        assert!((actual - expected).abs() < 1e-15);
    }

    #[test]
    fn global_solve_discount_curve_converges_beyond_1e8_pv_noise_floor() {
        let base_date = Date::from_calendar_date(2025, Month::December, 10).expect("base_date");
        let pricer =
            CalibrationPricer::new(base_date, "USD-OIS").with_market_conventions(Currency::USD);

        let deposit_conv = InstrumentConventions::default()
            .with_day_count(DayCount::Act360)
            .with_settlement_days(2)
            .with_calendar_id("usny");

        let ois_fixed_leg = InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360)
            .with_settlement_days(2)
            .with_payment_delay(2)
            .with_calendar_id("usny");

        let ois_float_leg = InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360)
            .with_reset_lag(0)
            .with_calendar_id("usny")
            .with_index("USD-SOFR-OIS");

        let quotes = vec![
            RatesQuote::Deposit {
                maturity: Date::from_calendar_date(2025, Month::December, 19).expect("mat"),
                rate: 0.0364447,
                conventions: deposit_conv.clone(),
            },
            RatesQuote::Deposit {
                maturity: Date::from_calendar_date(2026, Month::November, 12).expect("mat"),
                rate: 0.0345356,
                conventions: deposit_conv,
            },
            RatesQuote::Swap {
                maturity: Date::from_calendar_date(2026, Month::December, 14).expect("mat"),
                rate: 0.0343446,
                is_ois: true,
                conventions: InstrumentConventions::default(),
                fixed_leg_conventions: ois_fixed_leg.clone(),
                float_leg_conventions: ois_float_leg.clone(),
            },
            // 18M OIS introduces an intermediate stub coupon.
            RatesQuote::Swap {
                maturity: Date::from_calendar_date(2027, Month::June, 14).expect("mat"),
                rate: 0.0332849,
                is_ois: true,
                conventions: InstrumentConventions::default(),
                fixed_leg_conventions: ois_fixed_leg.clone(),
                float_leg_conventions: ois_float_leg.clone(),
            },
            RatesQuote::Swap {
                maturity: Date::from_calendar_date(2055, Month::December, 13).expect("mat"),
                rate: 0.0401000,
                is_ois: true,
                conventions: InstrumentConventions::default(),
                fixed_leg_conventions: ois_fixed_leg,
                float_leg_conventions: ois_float_leg,
            },
        ];

        let mut config = CalibrationConfig::default();
        config.solver = config.solver.with_tolerance(1e-12);
        config.calibration_method = crate::calibration::config::CalibrationMethod::GlobalSolve {
            use_analytical_jacobian: true,
        };

        let target = DiscountCurveTarget::new(DiscountCurveTargetParams {
            base_date,
            currency: Currency::USD,
            curve_id: CurveId::new("USD-OIS"),
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-OIS"),
            solve_interp: InterpStyle::MonotoneConvex,
            extrapolation: ExtrapolationPolicy::FlatForward,
            config,
            pricer,
            curve_day_count: DayCount::Act365F,
            spot_knot: None,
            settlement_date: base_date,
            base_context: MarketContext::new(),
        });

        let (_curve, report) =
            GlobalFitOptimizer::optimize(&target, &quotes, &target.config).expect("solve");
        assert!(
            report.max_residual < 1e-10,
            "expected GlobalSolve to fit beyond 1e-10; got max_residual={:.2e} (termination={:?})",
            report.max_residual,
            report.metadata.get("lm_termination_reason")
        );
    }
}

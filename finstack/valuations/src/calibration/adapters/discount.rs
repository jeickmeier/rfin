//! Discount curve calibration adapter.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::domain::pricing::CalibrationPricer;
use crate::calibration::domain::quotes::RatesQuote;
use crate::calibration::domain::solver::{BootstrapTarget, GlobalSolveTarget};
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
        if df_hi.is_finite() && df_hi > 0.0 && (df_hi - df_lo).abs() > 1e-10 {
            grid.push(df_hi);
        }

        let center = initial_df.clamp(df_lo, df_hi);
        let use_log_spacing = df_lo > 1e-12 && df_hi / df_lo > 10.0;

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
            let log_lo = df_lo.max(1e-12).ln();
            let log_hi = df_hi.max(1e-12).ln();
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
        grid.dedup_by(|a, b| (*a - *b).abs() < (df_hi - df_lo) * 0.001);
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
        self.curve_day_count
            .year_fraction(
                self.base_date,
                quote.maturity_date(),
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
            crate::calibration::config::RateBoundsPolicy::Explicit => {
                self.config.rate_bounds.min_rate < 0.0
            }
            crate::calibration::config::RateBoundsPolicy::AutoCurrency => {
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
                    .filter(|(ti, dfi)| *ti > 1e-8 && dfi.is_finite() && *dfi > 0.0)
                    .take(2)
                    .collect();

                if last_two.len() == 2 {
                    let (t1, df1) = last_two[0];
                    let (t0, df0) = last_two[1];
                    let dt = t1 - t0;
                    if dt > 1e-8 {
                        let f = (df0.ln() - df1.ln()) / dt;
                        let ln_df = df1.ln() - f * (t - t1);
                        let df = ln_df.exp();
                        if df.is_finite() && df > 0.0 {
                            guess = Some(df);
                        }
                    }
                } else if last_two.len() == 1 {
                    let (t1, df1) = last_two[0];
                    if t1 > 1e-8 {
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
            grid.dedup_by(|a, b| (*a - *b).abs() < (df_hi - df_lo) * 0.001);
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

        const DUPLICATE_TOL: f64 = 1e-10;
        let mut times = Vec::with_capacity(entries.len());
        let mut initials = Vec::with_capacity(entries.len());
        let mut active_quotes = Vec::with_capacity(entries.len());
        let mut last_time: Option<f64> = None;

        for (t, z, quote) in entries {
            if let Some(prev) = last_time {
                if (t - prev).abs() <= DUPLICATE_TOL {
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
            // DV01 grows roughly linearly with maturity; use sqrt to moderate long tenors.
            let weight = t.sqrt().max(1e-3);
            weights_out[i] = weight;
        }
        Ok(())
    }
}

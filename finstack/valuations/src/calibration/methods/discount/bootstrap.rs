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
use crate::calibration::methods::common::bootstrapper::BootstrapTarget;
use crate::calibration::pricing::{CalibrationPricer, RatesQuoteUseCase};
use crate::calibration::quotes::RatesQuote;
use crate::calibration::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::Solver;
use finstack_core::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const FALLBACK_INITIAL_DF: f64 = 0.95;

impl DiscountCurveCalibrator {
    pub(super) fn bootstrap_curve_with_solver<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        _solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Sort quotes by maturity
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(RatesQuote::maturity_date);

        // Validate quotes
        let bounds = self.config.effective_rate_bounds(self.currency);
        CalibrationPricer::validate_rates_quotes(
            &sorted_quotes,
            &bounds,
            self.base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: self.config.multi_curve.enforce_separation,
            },
        )?;

        // Setup
        let curve_dc = super::default_curve_day_count(self.currency);
        let pricer = self.create_pricer();
        pricer.validate_curve_dependencies(&sorted_quotes, base_context)?;
        let settlement = pricer.settlement_date(self.currency)?;

        // Spot knot
        let (t_spot, spot_knot) = self.compute_spot_knot(curve_dc, settlement);
        let mut initial_knots = Vec::with_capacity(2);
        initial_knots.push((0.0, 1.0));
        if let Some(knot) = spot_knot {
            initial_knots.push(knot);
        }

        // Create target
        let target = DiscountBootstrapper {
            calibrator: self,
            base_context: Rc::new(RefCell::new(base_context.clone())),
            pricer: &pricer,
            curve_dc,
            settlement,
        };

        // Run bootstrap with the initial knots (including optional spot knot)
        let (curve, report) =
            crate::calibration::methods::common::bootstrapper::SequentialBootstrapper::bootstrap(
                &target,
                &sorted_quotes,
                initial_knots,
                &self.config,
                None, // No explanation trace passed for now
            )?;

        // Add spot metadata which isn't in generic report
        let report = report
            .with_metadata("t_spot", format!("{:.6}", t_spot))
            .with_metadata("spot_knot_included", self.include_spot_knot.to_string())
            .with_metadata("curve_day_count", format!("{:?}", curve_dc))
            .with_metadata("solve_interp", format!("{:?}", self.solve_interp))
            .with_metadata("extrapolation", format!("{:?}", self.extrapolation))
            .with_metadata("currency", self.currency.to_string());

        // Validate final curve here (or generic could do it?)
        // Generic blindly calls build_curve. We want full validation.
        // `target.build_curve` does `build_for_solver`.
        // We might want to rebuild strictly at the end.
        // `SequentialBootstrapper` returns `T::Curve` from `target.build_curve`.
        // So we might need to rebuild "properly" here if `target.build_curve` is loose.
        // `DiscountBootstrapper::build_curve` will use `build_for_solver`.
        // So we should rebuild.

        // Re-extract knots from curve? Or just rebuild using curve knots.
        // DiscountCurve has knots() accessor.
        let correct_knots: Vec<(f64, f64)> = curve
            .knots()
            .iter()
            .zip(curve.dfs().iter())
            .map(|(t, v)| (*t, *v))
            .collect();
        self.build_final_curve_and_report(
            correct_knots,
            report.residuals,
            report.iterations,
            None,
            t_spot,
        )
    }

    // Helper reused for final build
    fn build_final_curve_and_report(
        &self,
        knots: Vec<(f64, f64)>,
        residuals: std::collections::BTreeMap<String, f64>,
        total_iterations: usize,
        trace: Option<finstack_core::explain::ExplanationTrace>,
        t_spot: f64,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        let curve = self.build_curve(
            self.curve_id.to_owned(),
            super::default_curve_day_count(self.currency),
            knots,
        )?;

        let mut validation_status = "passed";
        let mut validation_error: Option<String> = None;
        if let Err(e) = self.validate_calibrated_curve(&curve) {
            validation_status = "failed";
            validation_error = Some(e.to_string());
            match self.config.validation_mode {
                crate::calibration::config::ValidationMode::Warn => {
                    tracing::warn!("Calibrated discount curve failed validation: {}", e);
                }
                crate::calibration::config::ValidationMode::Error => return Err(e),
            }
        }

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
            format!("{:?}", super::default_curve_day_count(self.currency)),
        )
        .with_metadata("t_spot", format!("{:.6}", t_spot))
        .with_metadata("spot_knot_included", self.include_spot_knot.to_string())
        .with_metadata("validation", validation_status)
        .with_validation_result(validation_status == "passed", validation_error.clone());

        if let Some(err) = validation_error {
            report = report.with_metadata("validation_error", err);
        }
        if let Some(tr) = trace {
            report = report.with_explanation(tr);
        }

        Ok((curve, report))
    }
}

struct DiscountBootstrapper<'a> {
    calibrator: &'a DiscountCurveCalibrator,
    base_context: Rc<RefCell<MarketContext>>,
    pricer: &'a CalibrationPricer,
    curve_dc: finstack_core::dates::DayCount,
    settlement: finstack_core::dates::Date,
}

impl<'a> BootstrapTarget for DiscountBootstrapper<'a> {
    type Quote = RatesQuote;
    type Curve = DiscountCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let dc = quote.effective_day_count(self.calibrator.currency);
        dc.year_fraction(
            self.calibrator.base_date,
            quote.maturity_date(),
            finstack_core::dates::DayCountCtx::default(),
        )
        .map_err(|e| finstack_core::Error::Calibration {
            message: format!("YF calc failed: {}", e),
            category: "bootstrapping".to_string(),
        })
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        DiscountCurve::builder(self.calibrator.effective_discount_curve_id())
            .base_date(self.calibrator.base_date)
            .day_count(self.curve_dc)
            .knots(knots.iter().copied())
            .set_interp(self.calibrator.solve_interp)
            .allow_non_monotonic()
            .build_for_solver()
            .map_err(|_| finstack_core::Error::Calibration {
                message: "Failed to build temp curve".into(),
                category: "bootstrapping".to_string(),
            })
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        // Optimization: check suitability
        if quote.requires_forward_curve() && !quote.is_ois_suitable() {
            return Ok(crate::calibration::PENALTY);
        }

        {
            let mut ctx = self.base_context.borrow_mut();
            ctx.insert_mut(Arc::new(curve.clone()));
        }

        let ctx = self.base_context.borrow();
        let pv = self
            .pricer
            .price_instrument(quote, self.calibrator.currency, &ctx)
            .unwrap_or(crate::calibration::PENALTY);

        // Keep the signed residual so the root finder can detect sign changes.
        Ok(pv)
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> f64 {
        // Reuse logic from original helper
        // We need time_to_maturity
        let t = self.quote_time(quote).unwrap_or(1.0); // Should have been checked

        match quote {
            RatesQuote::Deposit { maturity, .. } => {
                let r = CalibrationPricer::get_rate(quote);
                let day_count = quote.effective_day_count(self.calibrator.currency);
                let yf = day_count
                    .year_fraction(
                        self.settlement,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(t)
                    .max(1e-6);
                1.0 / (1.0 + r * yf)
            }
            _ => {
                // Use last solved DF if available, otherwise derive from time
                if let Some((last_time, last_df)) = previous_knots.last() {
                    // Extrapolate using exponential decay
                    let time = self.quote_time(quote).unwrap_or(*last_time + 1.0);
                    let dt = time - *last_time;
                    if dt > 0.0 {
                        // Assume constant forward rate from last knot
                        let implied_rate = -(*last_df).ln() / (*last_time).max(1e-6);
                        (-implied_rate * time).exp()
                    } else {
                        *last_df
                    }
                } else {
                    // No previous knots - use fallback
                    FALLBACK_INITIAL_DF
                }
            }
        }
    }

    fn scan_points(&self, quote: &Self::Quote, initial_guess: f64) -> Vec<f64> {
        // Get time to maturity for proper bounds calculation
        let time = self.quote_time(quote).unwrap_or(1.0);
        
        // Get discount factor bounds for this maturity
        let (df_lo, df_hi) = self.calibrator.df_bounds_for_time(time);
        
        // Clamp initial guess to bounds
        let clamped_initial = initial_guess.clamp(df_lo, df_hi);
        
        // Use maturity-aware scan grid for robust root finding
        // Use more points (48 instead of 32) to ensure better coverage
        let mut grid = DiscountCurveCalibrator::maturity_aware_scan_grid(df_lo, df_hi, clamped_initial, 48);
        
        // Ensure we have enough points - if grid is too sparse, add more
        if grid.len() < 20 {
            // Add additional linear-spaced points across the range
            let num_additional = 20 - grid.len().min(20);
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
        
        grid
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
        // Optional bounds check
        let (lo, hi) = self.calibrator.df_bounds_for_time(time);
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

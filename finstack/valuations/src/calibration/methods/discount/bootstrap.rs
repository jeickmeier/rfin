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
use crate::calibration::pricing::conventions as conv;
use crate::calibration::quotes::RatesQuote;
use crate::calibration::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::*;

impl DiscountCurveCalibrator {
    pub(super) fn bootstrap_curve(
        &self,
        quotes: &[RatesQuote],
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
        let first_conv = sorted_quotes
            .first()
            .ok_or(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ))?
            .conventions();
        let first_settle = conv::resolve_common(&pricer, first_conv, self.currency);
        for q in sorted_quotes.iter().skip(1) {
            let s = conv::resolve_common(&pricer, q.conventions(), self.currency);
            if s.settlement_days != first_settle.settlement_days
                || s.calendar_id != first_settle.calendar_id
                || s.bdc != first_settle.bdc
            {
                return Err(finstack_core::Error::Validation(
                    "Inconsistent settlement conventions across discount curve quotes".to_string(),
                ));
            }
        }
        let settlement = pricer.settlement_date_for_quote(first_conv, self.currency)?;

        // Spot knot
        let (t_spot, spot_knot) = self.compute_spot_knot(curve_dc, settlement)?;
        let mut initial_knots = Vec::with_capacity(2);
        initial_knots.push((0.0, 1.0));
        if let Some(knot) = spot_knot {
            if knot.0 <= 0.0 {
                // Avoid duplicate 0.0 knot; builder requires strictly increasing times.
            } else {
            initial_knots.push(knot);
        }
        }

        // Create target
        let target = DiscountBootstrapper {
            calibrator: self,
            base_context: base_context.clone(),
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

        self.build_final_curve_and_report(
            curve,
            report.residuals,
            report.iterations,
            None,
            t_spot,
        )
    }

    // Helper reused for final build
    fn build_final_curve_and_report(
        &self,
        curve: DiscountCurve,
        residuals: std::collections::BTreeMap<String, f64>,
        total_iterations: usize,
        trace: Option<finstack_core::explain::ExplanationTrace>,
        t_spot: f64,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
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
    base_context: MarketContext,
    pricer: &'a CalibrationPricer,
    curve_dc: finstack_core::dates::DayCount,
    settlement: finstack_core::dates::Date,
}

impl<'a> BootstrapTarget for DiscountBootstrapper<'a> {
    type Quote = RatesQuote;
    type Curve = DiscountCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let dc = conv::resolve_money_market(self.pricer, quote.conventions(), self.calibrator.currency).day_count;
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
        let interp_style = if knots.len() < 3 {
            // Some interpolators (e.g., MonotoneConvex) need >=3 points; fallback for early knots.
            InterpStyle::Linear
        } else {
            self.calibrator.solve_interp
        };
        DiscountCurve::builder(self.calibrator.effective_discount_curve_id())
            .base_date(self.calibrator.base_date)
            .day_count(self.curve_dc)
            .knots(knots.iter().copied())
            .set_interp(interp_style)
            .allow_non_monotonic()
            .build_for_solver()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Failed to build temp curve: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // Use a simple interpolator during solving to avoid strict input requirements.
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
        let interp_style = InterpStyle::Linear;

        DiscountCurve::builder(self.calibrator.effective_discount_curve_id())
            .base_date(self.calibrator.base_date)
            .day_count(self.curve_dc)
            .knots(knots.iter().copied())
            .set_interp(interp_style)
            .allow_non_monotonic()
            .build_for_solver()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Failed to build temp curve: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.calibrator.build_curve(
            self.calibrator.curve_id.to_owned(),
            self.curve_dc,
            knots.to_vec(),
        )
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let temp_context = self.base_context.clone().insert_discount(curve.clone());
        let pv = self
            .pricer
            .price_instrument(quote, self.calibrator.currency, &temp_context)?;

        // Keep the signed residual so the root finder can detect sign changes.
        Ok(pv)
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let t = self.quote_time(quote)?;
        let (df_lo, df_hi) = self.calibrator.df_bounds_for_time(t);

        match quote {
            RatesQuote::Deposit { maturity, .. } => {
                let r = CalibrationPricer::get_rate(quote);
                let day_count = quote
                    .conventions()
                    .day_count
                    .ok_or_else(|| finstack_core::Error::Validation(
                        "Deposit quote requires conventions.day_count to be set".to_string(),
                    ))?;
                let yf = day_count
                    .year_fraction(
                        self.settlement,
                        *maturity,
                        finstack_core::dates::DayCountCtx::default(),
                    )?
                    .max(1e-6);
                let df = 1.0 / (1.0 + r * yf);
                Ok(df.clamp(df_lo, df_hi))
            }
            _ => {
                let mut guess = None;

                // Prefer log-linear extrapolation using the last two positive-time knots.
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
                    message: "Unable to derive initial DF guess (need at least one prior knot or deposit quote)".into(),
                    category: "bootstrapping".to_string(),
                })?;
                Ok(df.clamp(df_lo, df_hi))
            }
        }
    }

    fn scan_points(&self, quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        let time = self.quote_time(quote)?;
        
        // Get discount factor bounds for this maturity
        let (df_lo, df_hi) = self.calibrator.df_bounds_for_time(time);
        
        // Clamp initial guess to bounds
        let clamped_initial = initial_guess.clamp(df_lo, df_hi);
        
        let num_points = self.calibrator.config.discount_curve.scan_grid_points;
        let min_points = self.calibrator.config.discount_curve.min_scan_grid_points;
        let mut grid = DiscountCurveCalibrator::maturity_aware_scan_grid(
            df_lo,
            df_hi,
            clamped_initial,
            num_points,
        );
        
        // Ensure we have enough points - if grid is too sparse, add more
        if grid.len() < min_points {
            // Add additional linear-spaced points across the range
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

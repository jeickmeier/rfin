//! Forward curve calibration adapter.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::pricing::CalibrationPricer;
use crate::calibration::quotes::RatesQuote;
use crate::calibration::solver::BootstrapTarget;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};

/// Parameters for constructing a `ForwardCurveTarget`.
#[derive(Clone)]
pub struct ForwardCurveTargetParams {
    /// Base date for the curve (valuation date).
    pub base_date: Date,
    /// Currency of the forward curve (e.g. USD).
    pub currency: Currency,
    /// Unique identifier for the forward curve being calibrated.
    pub fwd_curve_id: CurveId,
    /// Identifier for the discount curve used for PV calculation.
    pub discount_curve_id: CurveId,
    /// Tenor associated with the forward rates (e.g. 3M, 6M).
    pub tenor_years: f64,
    /// Numerical interpolation style used during the solving process.
    pub solve_interp: InterpStyle,
    /// Global calibration settings (tolerances, rate bounds).
    pub config: CalibrationConfig,
    /// Factory for building pricing instruments from quotes.
    pub pricer: CalibrationPricer,
    /// Convention for converting dates to time axis (year fractions).
    pub time_day_count: DayCount,
    /// Context providing supporting market data (e.g. discount curves).
    pub base_context: MarketContext,
}

/// Target for forward curve calibration (Bootstrap).
///
/// This adapter bridges the [`SequentialBootstrapper`] with the forward rate
/// curve pricing logic. It handles knot anchor insertion at t=0 and provides
/// rate-bound aware scanning for numerical stability.
pub struct ForwardCurveTarget {
    /// Base date for the curve.
    pub base_date: Date,
    /// Currency of the curve.
    pub currency: Currency,
    /// Identifier for the forward curve being built.
    pub fwd_curve_id: CurveId,
    /// Identifier for the discount curve to use.
    pub discount_curve_id: CurveId,
    /// Tenor in years for the forward curve.
    pub tenor_years: f64,
    /// Interpolation style for solving.
    pub solve_interp: InterpStyle,
    /// Calibration configuration.
    pub config: CalibrationConfig,
    /// Pricer for calibration instruments.
    pub pricer: CalibrationPricer,
    /// Day count convention for time calculations.
    pub time_day_count: DayCount,
    /// Baseline market context.
    pub base_context: MarketContext,
}

impl ForwardCurveTarget {
    /// Create a new `ForwardCurveTarget` from parameters.
    pub fn new(params: ForwardCurveTargetParams) -> Self {
        Self {
            base_date: params.base_date,
            currency: params.currency,
            fwd_curve_id: params.fwd_curve_id,
            discount_curve_id: params.discount_curve_id,
            tenor_years: params.tenor_years,
            solve_interp: params.solve_interp,
            config: params.config,
            pricer: params.pricer,
            time_day_count: params.time_day_count,
            base_context: params.base_context,
        }
    }

    /// Calculate scale-aware tolerance for knot collision detection.
    pub fn scale_aware_tolerance(&self, knot_time: f64) -> f64 {
        let tol = self.config.solver.tolerance();
        (tol * (1.0 + knot_time)).max(tol)
    }
}

impl BootstrapTarget for ForwardCurveTarget {
    type Quote = RatesQuote;
    type Curve = ForwardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let knot_date = quote.maturity_date();
        self.time_day_count
            .year_fraction(self.base_date, knot_date, DayCountCtx::default())
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("YF calc failed: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // Time-grid logic should not depend on solver PV tolerance.
        const TIME_EPSILON: f64 = 1e-12;

        let mut full_knots = knots.to_vec();

        // Ensure anchor logic
        if full_knots.is_empty() {
            return Err(finstack_core::Error::Calibration {
                message: "Failed to build temp forward curve: need at least one knot".into(),
                category: "bootstrapping".to_string(),
            });
        } else {
            // If the first knot is not at (or extremely near) t=0, anchor the curve at t=0
            // using the first knot value. This ensures deterministic knot grids independent
            // of solver convergence thresholds.
            if full_knots[0].0 > TIME_EPSILON {
                full_knots.insert(0, (0.0, full_knots[0].1));
            }
        }

        ForwardCurve::builder(self.fwd_curve_id.clone(), self.tenor_years)
            .base_date(self.base_date)
            .knots(full_knots)
            .set_interp(self.solve_interp)
            .day_count(self.time_day_count)
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Failed to build temp forward curve: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let mut temp_context = self.base_context.clone();
        temp_context.insert_mut(std::sync::Arc::new(curve.clone()));

        let pv =
            self.pricer
                .price_instrument_for_calibration(quote, self.currency, &temp_context)?;
        Ok(pv)
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        match quote {
            RatesQuote::FRA { rate, .. } => Ok(*rate),
            RatesQuote::Future { price, specs, .. } => {
                let implied_rate = (100.0 - price) / 100.0;
                if let Some(adj) = specs.convexity_adjustment {
                    Ok(implied_rate + adj)
                } else {
                    Ok(implied_rate)
                }
            }
            RatesQuote::Swap { rate, .. } => Ok(*rate),
            _ => {
                let g = previous_knots.last().map(|(_, fwd)| *fwd).or_else(|| {
                    // Fallback to discount curve zero rate if available
                    let t = self.tenor_years.max(1.0 / 12.0);
                    self.base_context
                        .get_discount_ref(self.discount_curve_id.as_ref())
                        .ok()
                        .map(|disc_curve| disc_curve.zero(t))
                });
                g.ok_or_else(|| finstack_core::Error::Calibration {
                    message: "Unable to derive initial forward rate guess".into(),
                    category: "bootstrapping".to_string(),
                })
            }
        }
    }

    fn scan_points(&self, _quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        let bounds = self.config.effective_rate_bounds(self.currency);
        let center = if initial_guess.is_finite() {
            initial_guess.clamp(bounds.min_rate, bounds.max_rate)
        } else {
            0.0_f64.clamp(bounds.min_rate, bounds.max_rate)
        };

        // Bounded geometric expansion around the initial guess.
        // This avoids hard-coded scan grids while keeping the search within
        // the configured rate bounds.
        let step0 = (1e-4 * (1.0 + center.abs())).max(1e-8);
        let mut step = step0;

        let mut pts = Vec::with_capacity(2 * 16 + 3);
        pts.push(bounds.min_rate);
        pts.push(center);
        pts.push(bounds.max_rate);

        for _ in 0..16 {
            pts.push((center - step).clamp(bounds.min_rate, bounds.max_rate));
            pts.push((center + step).clamp(bounds.min_rate, bounds.max_rate));
            step *= 2.0;
        }

        pts.sort_by(|a, b| a.total_cmp(b));
        pts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        Ok(pts)
    }

    fn validate_knot(&self, time: f64, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!("Non-finite forward rate at t={:.6}", time),
                category: "bootstrapping".to_string(),
            });
        }
        let bounds = self.config.effective_rate_bounds(self.currency);
        if !bounds.contains(value) {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Solved forward rate out of bounds for {} at t={:.6}: {:.4}%",
                    self.fwd_curve_id,
                    time,
                    value * 100.0
                ),
                category: "bootstrapping".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::solver::BootstrapTarget;
    use time::Month;

    #[test]
    fn forward_curve_anchor_insertion_independent_of_solver_tolerance() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let currency = Currency::USD;
        let fwd_curve_id = CurveId::new("fwd");
        let discount_curve_id = CurveId::new("disc");

        let pricer = CalibrationPricer::for_forward_curve(
            base_date,
            fwd_curve_id.clone(),
            discount_curve_id.clone(),
            1.0,
        );

        let mk_target = |tolerance: f64| ForwardCurveTarget {
            base_date,
            currency,
            fwd_curve_id: fwd_curve_id.clone(),
            discount_curve_id: discount_curve_id.clone(),
            tenor_years: 1.0,
            solve_interp: InterpStyle::Linear,
            config: CalibrationConfig {
                solver: crate::calibration::solver::SolverConfig::brent_default()
                    .with_tolerance(tolerance),
                ..CalibrationConfig::default()
            },
            pricer: pricer.clone(),
            time_day_count: DayCount::Act365F,
            base_context: MarketContext::new(),
        };

        // Choose a small but realistic first time > 0; old code would conditionally add the
        // anchor depending on solver tolerance.
        let knots = vec![(1e-6, 0.01), (1.0, 0.02)];

        let low_tol_curve = mk_target(1e-10)
            .build_curve(&knots)
            .expect("curve build should succeed");
        let high_tol_curve = mk_target(5e-1)
            .build_curve(&knots)
            .expect("curve build should succeed");

        assert_eq!(low_tol_curve.knots(), high_tol_curve.knots());
        assert_eq!(low_tol_curve.knots(), &[0.0, 1e-6, 1.0]);
    }
}

//! Forward curve calibration adapter.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::domain::pricing::CalibrationPricer;
use crate::calibration::v2::domain::quotes::RatesQuote;
use crate::calibration::v2::domain::solver::BootstrapTarget;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};

/// Parameters for constructing a `ForwardCurveTarget`.
#[derive(Clone)]
pub struct ForwardCurveTargetParams {
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
    /// Context needed for pricing against OTHER curves (if any).
    pub base_context: MarketContext,
}

/// Target for forward curve calibration (Bootstrap).
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
    /// Context needed for pricing against OTHER curves (if any).
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
        (self.config.tolerance * (1.0 + knot_time)).max(self.config.tolerance)
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
        let mut full_knots = knots.to_vec();

        // Ensure anchor logic
        if full_knots.is_empty() {
            full_knots.push((0.0, 0.02)); // Fallback if strictly empty
        } else {
            // Logic from ensure_anchor: derive from first knot if > tolerance
            if full_knots[0].0 > self.config.tolerance {
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

        let pv = self
            .pricer
            .price_instrument(quote, self.currency, &temp_context)?;
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
        let mut grid = vec![
            -0.10, -0.05, -0.03, -0.02, -0.01, -0.005, 0.0, 0.002, 0.005, 0.01, 0.015, 0.02, 0.025,
            0.03, 0.035, 0.04, 0.045, 0.05, 0.06, 0.075, 0.10, 0.125, 0.15, 0.20, 0.25, 0.30, 0.40,
            0.50,
        ];

        // Add initial guess
        grid.push(initial_guess);

        // Filter to bounds
        let filtered: Vec<f64> = grid
            .into_iter()
            .filter(|&r| r >= bounds.min_rate - 0.05 && r <= bounds.max_rate + 0.05)
            .collect();

        let mut res = filtered;
        res.sort_by(|a, b| a.total_cmp(b));
        res.dedup();
        Ok(res)
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

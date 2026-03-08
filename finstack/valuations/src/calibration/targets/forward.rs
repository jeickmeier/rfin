//! Forward curve calibration target.

use crate::calibration::api::schema::ForwardCurveParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::config::CalibrationMethod;
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::bootstrap::SequentialBootstrapper;
use crate::calibration::solver::traits::BootstrapTarget;
use crate::calibration::targets::util::curve_day_count_from_quotes;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::cell::RefCell;

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
    /// Day count convention for time calculations.
    pub time_day_count: DayCount,
    /// Baseline market context.
    pub base_context: MarketContext,
    /// Optional reusable context for sequential solvers to reduce memory pressure.
    reuse_context: Option<RefCell<MarketContext>>,
}

impl ForwardCurveTarget {
    /// Create a new `ForwardCurveTarget` from parameters.
    pub fn new(params: ForwardCurveTargetParams) -> Self {
        let reuse_context = if params.config.use_parallel {
            None
        } else {
            Some(RefCell::new(params.base_context.clone()))
        };
        Self {
            base_date: params.base_date,
            currency: params.currency,
            fwd_curve_id: params.fwd_curve_id,
            discount_curve_id: params.discount_curve_id,
            tenor_years: params.tenor_years,
            solve_interp: params.solve_interp,
            config: params.config,
            time_day_count: params.time_day_count,
            base_context: params.base_context,
            reuse_context,
        }
    }

    fn with_temp_context<F, T>(&self, curve: &ForwardCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();
            *ctx = std::mem::take(&mut *ctx).insert(curve.clone());
            op(&ctx)
        } else {
            let mut temp_context = self.base_context.clone();
            temp_context = temp_context.insert(curve.clone());
            op(&temp_context)
        }
    }

    /// Execute the full calibration for a forward curve step.
    pub fn solve(
        params: &ForwardCurveParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let rates_quotes: Vec<crate::market::quotes::rates::RateQuote> = quotes.extract_quotes();

        if rates_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let curve_dc = curve_day_count_from_quotes(&rates_quotes)?;

        let mut curve_ids = finstack_core::HashMap::default();
        curve_ids.insert("discount".to_string(), params.discount_curve_id.to_string());
        curve_ids.insert("forward".to_string(), params.curve_id.to_string());

        let build_ctx =
            crate::market::build::context::BuildCtx::new(params.base_date, 1.0, curve_ids);

        let mut prepared_quotes: Vec<CalibrationQuote> = Vec::with_capacity(rates_quotes.len());

        let pillar_policy = crate::market::build::prepared::PillarPolicy::default();
        for q in rates_quotes {
            let prepared = crate::market::build::prepared::prepare_rate_quote(
                q,
                &build_ctx,
                curve_dc,
                params.base_date,
                &pillar_policy,
            )?;
            prepared_quotes.push(CalibrationQuote::Rates(prepared));
        }

        let mut config = global_config.clone();
        config.calibration_method = params.method.clone();

        let target = ForwardCurveTarget::new(ForwardCurveTargetParams {
            base_date: params.base_date,
            currency: params.currency,
            fwd_curve_id: params.curve_id.clone(),
            discount_curve_id: params.discount_curve_id.clone(),
            tenor_years: params.tenor_years,
            solve_interp: params.interpolation,
            config: config.clone(),
            time_day_count: curve_dc,
            base_context: context.clone(),
        });

        // Forward curves use discount curve validation tolerance (could add dedicated config later).
        let success_tolerance = Some(config.discount_curve.validation_tolerance);

        let (curve, mut report) = match params.method {
            CalibrationMethod::Bootstrap => SequentialBootstrapper::bootstrap(
                &target,
                &prepared_quotes,
                Vec::new(),
                &config,
                success_tolerance,
                None,
            )?,
            CalibrationMethod::GlobalSolve { .. } => {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ));
            }
        };

        report.update_solver_config(config.solver.clone());

        let new_context = context.clone().insert(curve);
        Ok((new_context, report))
    }
}

impl BootstrapTarget for ForwardCurveTarget {
    type Quote = crate::calibration::prepared::CalibrationQuote;
    type Curve = ForwardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        match quote {
            crate::calibration::prepared::CalibrationQuote::Rates(pq) => Ok(pq.pillar_time),
            _ => Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            )),
        }
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
            .interp(self.solve_interp)
            .day_count(self.time_day_count)
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!("Failed to build temp forward curve: {}", e),
                category: "bootstrapping".to_string(),
            })
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let pq = match quote {
            crate::calibration::prepared::CalibrationQuote::Rates(pq) => pq,
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ))
            }
        };
        self.with_temp_context(curve, |ctx| {
            let pv = pq.instrument.value_raw(ctx, self.base_date)?;
            Ok(pv / 1.0)
        })
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let pq = match quote {
            crate::calibration::prepared::CalibrationQuote::Rates(pq) => pq,
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ))
            }
        };
        let q = pq.quote.as_ref();
        use crate::market::quotes::rates::RateQuote;
        match q {
            RateQuote::Fra { rate, .. } => Ok(*rate),
            RateQuote::Futures {
                price,
                convexity_adjustment,
                vol_surface_id,
                ..
            } => {
                if vol_surface_id.is_some() && convexity_adjustment.is_none() {
                    return Err(finstack_core::Error::Validation(
                        "Forward curve calibration requires a pre-computed convexity_adjustment \
                         for futures quotes; dynamic vol-surface lookup is not wired"
                            .to_string(),
                    ));
                }
                let implied_rate = (100.0 - price) / 100.0;
                if let Some(adj) = convexity_adjustment {
                    Ok(implied_rate - adj)
                } else {
                    Ok(implied_rate)
                }
            }
            RateQuote::Swap { rate, .. } => Ok(*rate),
            _ => {
                let g = previous_knots.last().map(|(_, fwd)| *fwd).or_else(|| {
                    // Fallback to discount curve zero rate if available
                    let t = self.tenor_years.max(1.0 / 12.0);
                    self.base_context
                        .get_discount(self.discount_curve_id.as_ref())
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
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::calibration::prepared::CalibrationQuote;
    use crate::calibration::solver::traits::BootstrapTarget;
    use crate::instruments::common_impl::traits::{Attributes, Instrument};
    use crate::market::build::prepared::PreparedQuote;
    use crate::market::conventions::ids::IrFutureContractId;
    use crate::market::quotes::ids::QuoteId;
    use crate::market::quotes::rates::RateQuote;
    use crate::pricer::InstrumentType;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use std::any::Any;
    use std::sync::Arc;
    use time::Month;

    #[derive(Clone)]
    struct DummyInstrument;

    impl Instrument for DummyInstrument {
        fn id(&self) -> &str {
            "dummy"
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Deposit
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
            Ok(Money::new(0.0, Currency::USD))
        }

        fn attributes(&self) -> &Attributes {
            static ATTRS: std::sync::OnceLock<Attributes> = std::sync::OnceLock::new();
            ATTRS.get_or_init(Attributes::default)
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            unreachable!("dummy instrument should not mutate attributes")
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn forward_curve_anchor_insertion_independent_of_solver_tolerance() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let currency = Currency::USD;
        let fwd_curve_id = CurveId::new("fwd");
        let discount_curve_id = CurveId::new("disc");

        let mk_target = |tolerance: f64| {
            let base_context = MarketContext::new();
            let config = CalibrationConfig {
                solver: crate::calibration::solver::SolverConfig::brent_default()
                    .with_tolerance(tolerance),
                ..CalibrationConfig::default()
            };
            let reuse_context = if config.use_parallel {
                None
            } else {
                Some(RefCell::new(base_context.clone()))
            };
            ForwardCurveTarget {
                base_date,
                currency,
                fwd_curve_id: fwd_curve_id.clone(),
                discount_curve_id: discount_curve_id.clone(),
                tenor_years: 1.0,
                solve_interp: InterpStyle::Linear,
                config,
                time_day_count: DayCount::Act365F,
                base_context,
                reuse_context,
            }
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

    #[test]
    fn futures_initial_guess_subtracts_convexity_adjustment() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let target = ForwardCurveTarget {
            base_date,
            currency: Currency::USD,
            fwd_curve_id: CurveId::new("fwd"),
            discount_curve_id: CurveId::new("disc"),
            tenor_years: 1.0,
            solve_interp: InterpStyle::Linear,
            config: CalibrationConfig::default(),
            time_day_count: DayCount::Act365F,
            base_context: MarketContext::new(),
            reuse_context: Some(RefCell::new(MarketContext::new())),
        };

        let quote = CalibrationQuote::Rates(PreparedQuote::new(
            Arc::new(RateQuote::Futures {
                id: QuoteId::new("SR3"),
                contract: IrFutureContractId::new("CME:SR3"),
                expiry: base_date,
                price: 98.50,
                convexity_adjustment: Some(0.0010),
                vol_surface_id: None,
            }),
            Arc::new(DummyInstrument),
            base_date,
            1.0,
        ));

        let guess = target.initial_guess(&quote, &[]).expect("initial guess");
        assert!((guess - 0.014).abs() < 1e-12, "expected 1.40%, got {guess}");
    }

    #[test]
    fn futures_initial_guess_rejects_unwired_dynamic_convexity_shape() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let target = ForwardCurveTarget {
            base_date,
            currency: Currency::USD,
            fwd_curve_id: CurveId::new("fwd"),
            discount_curve_id: CurveId::new("disc"),
            tenor_years: 1.0,
            solve_interp: InterpStyle::Linear,
            config: CalibrationConfig::default(),
            time_day_count: DayCount::Act365F,
            base_context: MarketContext::new(),
            reuse_context: Some(RefCell::new(MarketContext::new())),
        };

        let quote = CalibrationQuote::Rates(PreparedQuote::new(
            Arc::new(RateQuote::Futures {
                id: QuoteId::new("SR3"),
                contract: IrFutureContractId::new("CME:SR3"),
                expiry: base_date,
                price: 98.50,
                convexity_adjustment: None,
                vol_surface_id: Some(CurveId::new("USD-SR3-VOL")),
            }),
            Arc::new(DummyInstrument),
            base_date,
            1.0,
        ));

        let err = target
            .initial_guess(&quote, &[])
            .expect_err("unsupported dynamic convexity shape should fail closed");
        assert!(err
            .to_string()
            .contains("pre-computed convexity_adjustment"));
    }
}

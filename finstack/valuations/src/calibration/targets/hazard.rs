use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::config::{CalibrationConfig, CalibrationMethod, ResidualWeightingScheme};
use crate::calibration::constants::WEIGHT_MIN_FLOOR;
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::{
    BootstrapTarget, GlobalFitOptimizer, GlobalSolveTarget, SequentialBootstrapper,
};
use crate::calibration::targets::util::sort_bootstrap_quotes;
use crate::calibration::CalibrationReport;
use crate::instruments::cds::CdsConventionResolved;
use crate::market::build::context::BuildCtx;
use crate::market::build::prepared::PreparedQuote;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use finstack_core::collections::HashMap;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::Result;
use std::cell::RefCell;

const HAZARD_HARD_MIN: f64 = 0.0;
// Safety cap: λ=10 implies ~99.995% 1Y default probability and can lead to numerical underflow
// in long-dated curves. Treat as a hard validation error during calibration.
const HAZARD_HARD_MAX: f64 = 10.0;
const TOLERANCE_DUP_KNOTS: f64 = 1e-12;

/// Bootstrapper for hazard curves from CDS quotes.
///
/// Implements sequential bootstrapping of hazard curves (survival probabilities)
/// using market CDS quotes with varying maturities. It derives standard ISDA
/// conventions (e.g., North American, European, Asian) from the currency and
/// prices synthetic CDS instruments to solve for the hazard rate at each knot.
///
/// # Invariants
/// - Hazard rates must be non-negative (to ensure non-increasing survival).
/// - Knot times must be strictly increasing.
///
/// # See Also
/// - [`crate::instruments::cds`] for details on the underlying instruments.
pub struct HazardBootstrapper {
    /// Parameters defining the hazard curve structure and IDs.
    pub params: HazardCurveParams,
    /// CDS market conventions resolved from (currency, doc_clause).
    pub(crate) cds_conventions: &'static CdsConventionResolved,
    /// Market context providing discount curves for PV calculations.
    pub base_context: MarketContext,
    /// Global calibration settings (used for solver controls and weights).
    pub config: CalibrationConfig,
    /// Optional reusable context for sequential solvers to reduce memory pressure.
    reuse_context: Option<RefCell<MarketContext>>,
}

impl HazardBootstrapper {
    /// Creates a new hazard curve bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the hazard curve structure
    /// * `base_context` - Market context containing discount curves
    /// * `config` - Global calibration settings (solver controls and weights)
    ///
    /// # Returns
    ///
    /// A new `HazardBootstrapper` instance ready for calibration.
    ///
    /// # Note
    ///
    /// CDS conventions are automatically derived from the currency:
    /// - USD/CAD: ISDA North American
    /// - EUR/GBP/CHF: ISDA European
    /// - JPY/HKD/SGD/AUD/NZD: ISDA Asian
    pub fn new(
        params: HazardCurveParams,
        base_context: MarketContext,
        config: CalibrationConfig,
    ) -> Result<Self> {
        let cds_conventions = crate::instruments::cds::resolve_market_conventions(
            params.currency,
            params.doc_clause.as_deref(),
        )?;

        let reuse_context = if config.use_parallel {
            None
        } else {
            Some(RefCell::new(base_context.clone()))
        };

        Ok(Self {
            params,
            cds_conventions,
            base_context,
            config,
            reuse_context,
        })
    }

    /// Execute the full calibration for a hazard curve step.
    pub fn solve(
        params: &HazardCurveParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let cds_quotes: Vec<crate::market::quotes::cds::CdsQuote> = quotes.extract_quotes();

        if cds_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        let target =
            HazardBootstrapper::new(params.clone(), context.clone(), global_config.clone())?;

        let mut prepared_quotes: Vec<CalibrationQuote> = Vec::with_capacity(cds_quotes.len());
        let mut curve_ids = HashMap::default();
        curve_ids.insert("discount".to_string(), params.discount_curve_id.to_string());
        curve_ids.insert("credit".to_string(), params.curve_id.to_string());
        let build_ctx = BuildCtx::new(params.base_date, params.notional, curve_ids);

        for (i, q) in cds_quotes.into_iter().enumerate() {
            // Build Instrument
            let instrument = crate::market::build::cds::build_cds_instrument(&q, &build_ctx)
                .map_err(|e| {
                    finstack_core::Error::Validation(format!(
                        "Failed to build credit instrument {}: {}",
                        i, e
                    ))
                })?;
            let instrument: std::sync::Arc<dyn crate::instruments::common::traits::Instrument> =
                instrument.into();

            let maturity_date = if let Some(cds) = instrument
                .as_any()
                .downcast_ref::<crate::instruments::cds::CreditDefaultSwap>(
            ) {
                cds.premium.end
            } else {
                return Err(finstack_core::Error::Validation(
                    "Expected CreditDefaultSwap instrument".into(),
                ));
            };

            let t_day_count = target.cds_conventions.day_count;
            let pillar_time = t_day_count.year_fraction(
                params.base_date,
                maturity_date,
                DayCountCtx::default(),
            )?;

            let prepared = PreparedQuote::new(
                std::sync::Arc::new(q.clone()),
                instrument,
                maturity_date,
                pillar_time,
            );

            prepared_quotes.push(CalibrationQuote::Cds(prepared, None));
        }

        let (curve, report) = match params.method {
            CalibrationMethod::Bootstrap => {
                sort_bootstrap_quotes(&target, &mut prepared_quotes)?;
                SequentialBootstrapper::bootstrap(
                    &target,
                    &prepared_quotes,
                    Vec::new(),
                    global_config,
                    None,
                )?
            }
            CalibrationMethod::GlobalSolve { .. } => {
                GlobalFitOptimizer::optimize(&target, &prepared_quotes, global_config)?
            }
        };

        let mut report = report;
        report.update_solver_config(global_config.solver.clone());

        let mut new_context = context.clone();
        new_context.insert_hazard_mut(curve);
        Ok((new_context, report))
    }

    fn quote_hazard_guess(&self, quote: &CalibrationQuote) -> Option<f64> {
        let pq = match quote {
            CalibrationQuote::Cds(pq, _) => pq,
            _ => return None,
        };

        let (spread_bp, recovery) = match pq.quote.as_ref() {
            crate::market::quotes::cds::CdsQuote::CdsParSpread {
                spread_bp,
                recovery_rate,
                ..
            } => (*spread_bp, *recovery_rate),
            crate::market::quotes::cds::CdsQuote::CdsUpfront {
                running_spread_bp,
                recovery_rate,
                ..
            } => (*running_spread_bp, *recovery_rate),
        };

        let loss_given_default = (1.0 - recovery).max(1e-6);
        let guess = (spread_bp / 10_000.0) / loss_given_default;
        if guess.is_finite() && guess >= 0.0 {
            Some(guess.clamp(HAZARD_HARD_MIN, HAZARD_HARD_MAX))
        } else {
            None
        }
    }

    fn with_temp_context<F, T>(&self, curve: &HazardCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();
            ctx.insert_hazard_mut(curve.clone());
            // Sync CreditIndex if it exists (so pricer sees trial curve)
            if let Ok(idx) = ctx.credit_index_ref(&self.params.curve_id) {
                let mut updated = idx.clone();
                updated.index_credit_curve = std::sync::Arc::new(curve.clone());
                ctx.insert_credit_index_mut(&self.params.curve_id, updated);
            }
            op(&ctx)
        } else {
            let mut temp_context = self.base_context.clone();
            temp_context.insert_hazard_mut(curve.clone());
            // Sync CreditIndex if it exists
            if let Ok(idx) = temp_context.credit_index_ref(&self.params.curve_id) {
                let mut updated = idx.clone();
                updated.index_credit_curve = std::sync::Arc::new(curve.clone());
                temp_context.insert_credit_index_mut(&self.params.curve_id, updated);
            }
            op(&temp_context)
        }
    }
}

impl BootstrapTarget for HazardBootstrapper {
    type Quote = crate::calibration::prepared::CalibrationQuote;
    type Curve = HazardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        match quote {
            crate::calibration::prepared::CalibrationQuote::Cds(pq, _) => Ok(pq.pillar_time),
            _ => Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        HazardCurve::builder(self.params.curve_id.to_string())
            .base_date(self.params.base_date)
            .day_count(self.cds_conventions.day_count)
            .issuer(self.params.entity.clone())
            .seniority(self.params.seniority)
            .currency(self.params.currency)
            .recovery_rate(self.params.recovery_rate)
            .knots(knots.to_vec())
            // Par spread interpolation is for *reporting* quoted spreads on the calibrated curve.
            // Positivity / no-arbitrage for survival is enforced via λ>=0 and the curve's
            // log-linear survival interpolation (in finstack_core).
            .par_interp(self.params.par_interp)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let pq = match quote {
            crate::calibration::prepared::CalibrationQuote::Cds(pq, _) => pq,
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };
        let base_date = self.params.base_date;
        self.with_temp_context(curve, |ctx| {
            let npv = pq.instrument.value_raw(ctx, base_date)?;
            Ok(npv / self.params.notional)
        })
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let guess = previous_knots.last().map(|&(_, v)| v).unwrap_or(0.01);
        if guess.is_finite() {
            Ok(guess.clamp(HAZARD_HARD_MIN, HAZARD_HARD_MAX))
        } else {
            Ok(0.01)
        }
    }

    fn scan_points(&self, _quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        // Bounded, maturity-agnostic scan grid (log-spaced) on [0, HAZARD_HARD_MAX].
        // This prevents the solver from spending effort in negative/absurd hazard regions.
        let max_h = HAZARD_HARD_MAX;
        let min_positive = 1e-10_f64;

        let center = if initial_guess.is_finite() {
            initial_guess.clamp(HAZARD_HARD_MIN, max_h)
        } else {
            0.01_f64
        };

        let mut pts = Vec::with_capacity(64);
        pts.push(0.0);
        pts.push(center);
        pts.push(max_h);

        let center_pos = center.max(min_positive);
        let log_center = center_pos.log10();
        let low_exp = (log_center - 4.0).max(min_positive.log10());
        let high_exp = (log_center + 2.0).min(max_h.log10());

        const N: usize = 48;
        if (high_exp - low_exp).abs() > 1e-12 {
            for i in 0..N {
                let t = i as f64 / (N - 1) as f64;
                let exp = low_exp + t * (high_exp - low_exp);
                let v = 10f64.powf(exp);
                if v.is_finite() && v >= 0.0 && v <= max_h {
                    pts.push(v);
                }
            }
        } else {
            pts.push(center_pos);
        }

        pts.sort_by(|a, b| a.total_cmp(b));
        pts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        Ok(pts)
    }

    fn validate_knot(&self, time: f64, value: f64) -> Result<()> {
        if !time.is_finite() || time <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Invalid hazard knot time for {}: t={}",
                    self.params.curve_id, time
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if !value.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Non-finite hazard rate for {} at t={:.6}",
                    self.params.curve_id, time
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if value < HAZARD_HARD_MIN {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Negative hazard rate for {} at t={:.6}: {:.6}",
                    self.params.curve_id, time, value
                ),
                category: "bootstrapping".to_string(),
            });
        }
        if value > HAZARD_HARD_MAX {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Hazard rate out of bounds for {} at t={:.6}: {:.6} (max {:.6})",
                    self.params.curve_id, time, value, HAZARD_HARD_MAX
                ),
                category: "bootstrapping".to_string(),
            });
        }
        Ok(())
    }
}

impl GlobalSolveTarget for HazardBootstrapper {
    type Quote = CalibrationQuote;
    type Curve = HazardCurve;

    fn build_time_grid_and_guesses(
        &self,
        quotes: &[Self::Quote],
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<Self::Quote>)> {
        let seed_curve = self
            .base_context
            .get_hazard(self.params.curve_id.as_str())
            .ok();

        let mut entries = Vec::with_capacity(quotes.len());

        for quote in quotes {
            let t = self.quote_time(quote)?;
            if !t.is_finite() || t <= 0.0 {
                continue;
            }

            let guess = if let Some(curve) = seed_curve.as_ref() {
                curve.hazard_rate(t)
            } else {
                self.quote_hazard_guess(quote).unwrap_or(0.01)
            };

            let guess = if guess.is_finite() {
                guess.clamp(HAZARD_HARD_MIN, HAZARD_HARD_MAX)
            } else {
                0.01
            };

            entries.push((t, guess, quote.clone()));
        }

        entries.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut times = Vec::with_capacity(entries.len());
        let mut initials = Vec::with_capacity(entries.len());
        let mut active_quotes = Vec::with_capacity(entries.len());
        let mut last_time: Option<f64> = None;

        for (t, guess, quote) in entries {
            if let Some(prev) = last_time {
                if (t - prev).abs() <= TOLERANCE_DUP_KNOTS {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Duplicate or unsorted hazard knot times detected (prev={:.10}, new={:.10}). \
Ensure quotes map to strictly increasing year fractions.",
                            prev, t
                        ),
                        category: "global_solve".to_string(),
                    });
                }
            }
            last_time = Some(t);
            times.push(t);
            initials.push(guess);
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

        let mut knots = Vec::with_capacity(times.len());
        let mut last_t = 0.0;

        for (&t, &lambda) in times.iter().zip(params.iter()) {
            if t <= last_t {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Non-increasing hazard knot time {:.10} detected (previous {:.10}). \
Global solve requires strictly increasing times.",
                        t, last_t
                    ),
                    category: "global_solve".to_string(),
                });
            }
            self.validate_knot(t, lambda)?;
            last_t = t;
            knots.push((t, lambda));
        }

        self.build_curve_for_solver(&knots)
    }

    fn build_curve_final_from_params(&self, times: &[f64], params: &[f64]) -> Result<Self::Curve> {
        self.build_curve_for_solver_from_params(times, params)
    }

    fn calculate_residuals(
        &self,
        curve: &Self::Curve,
        quotes: &[Self::Quote],
        residuals: &mut [f64],
    ) -> Result<()> {
        if residuals.len() < quotes.len() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve residuals buffer too small: got {} need {}",
                    residuals.len(),
                    quotes.len()
                ),
                category: "global_solve".to_string(),
            });
        }

        self.with_temp_context(curve, |ctx| {
            for (i, quote) in quotes.iter().enumerate() {
                let pq = match quote {
                    CalibrationQuote::Cds(pq, _) => pq,
                    _ => {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ))
                    }
                };
                let npv = pq.instrument.value_raw(ctx, self.params.base_date)?;
                residuals[i] = npv / self.params.notional;
            }
            Ok(())
        })
    }

    fn residual_key(&self, quote: &Self::Quote, idx: usize) -> String {
        let q = quote.instrument();
        format!("{}-{:03}", q.id(), idx)
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
                ResidualWeightingScheme::InverseDuration => 1.0 / t.max(0.1),
            };

            weights_out[i] = weight.max(WEIGHT_MIN_FLOOR);
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::calibration::solver::BootstrapTarget;
    use finstack_core::dates::Date;

    use finstack_core::market_data::term_structures::ParInterp;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::types::{Currency, CurveId};
    use time::Month;

    fn base_params() -> HazardCurveParams {
        HazardCurveParams {
            curve_id: CurveId::new("TEST-HAZ".to_string()),
            entity: "ACME".to_string(),
            seniority: finstack_core::market_data::term_structures::Seniority::Senior,
            currency: Currency::USD,
            base_date: Date::from_calendar_date(2025, Month::January, 1).expect("valid base_date"),
            discount_curve_id: CurveId::new("USD-OIS".to_string()),
            recovery_rate: 0.4,
            notional: 1.0,
            method: crate::calibration::config::CalibrationMethod::Bootstrap,
            interpolation: InterpStyle::Linear,
            par_interp: ParInterp::Linear,
            doc_clause: None,
        }
    }

    #[test]
    fn validate_knot_rejects_negative_hazard() {
        let target = HazardBootstrapper::new(
            base_params(),
            MarketContext::default(),
            CalibrationConfig::default(),
        )
        .expect("target");
        let err = target
            .validate_knot(1.0, -1e-6)
            .expect_err("should reject negative hazard");
        assert!(err.to_string().to_lowercase().contains("negative hazard"));
    }

    #[test]
    fn validate_knot_rejects_hazard_above_max() {
        let target = HazardBootstrapper::new(
            base_params(),
            MarketContext::default(),
            CalibrationConfig::default(),
        )
        .expect("target");
        let err = target
            .validate_knot(1.0, HAZARD_HARD_MAX + 1e-6)
            .expect_err("should reject excessive hazard");
        assert!(err.to_string().to_lowercase().contains("out of bounds"));
    }

    #[test]
    fn build_curve_preserves_par_interp_and_monotone_survival() {
        let mut p = base_params();
        p.par_interp = ParInterp::LogLinear;
        let target =
            HazardBootstrapper::new(p, MarketContext::default(), CalibrationConfig::default())
                .expect("target");

        let curve = target
            .build_curve(&[(1.0, 0.02), (5.0, 0.03)])
            .expect("curve build should succeed");
        assert_eq!(curve.par_interp(), ParInterp::LogLinear);

        let s1 = curve.sp(1.0);
        let s5 = curve.sp(5.0);
        let s10 = curve.sp(10.0);
        assert!((0.0..=1.0).contains(&s1));
        assert!((0.0..=1.0).contains(&s5));
        assert!((0.0..=1.0).contains(&s10));
        assert!(s1 >= s5 && s5 >= s10);
    }
}

use crate::calibration::api::schema::HazardCurveParams;
use crate::calibration::config::{CalibrationConfig, CalibrationMethod};
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::{BootstrapTarget, SequentialBootstrapper};
use crate::calibration::targets::util::sort_bootstrap_quotes;
use crate::calibration::CalibrationReport;
use crate::instruments::cds::CdsConventionResolved;
use crate::market::build::context::BuildCtx;
use crate::market::build::prepared::PreparedQuote;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::Result;
use std::cell::RefCell;
use std::collections::HashMap;

const HAZARD_HARD_MIN: f64 = 0.0;
// Safety cap: λ=10 implies ~99.995% 1Y default probability and can lead to numerical underflow
// in long-dated curves. Treat as a hard validation error during calibration.
const HAZARD_HARD_MAX: f64 = 10.0;

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
        use_parallel: bool,
    ) -> Result<Self> {
        let cds_conventions = crate::instruments::cds::resolve_market_conventions(
            params.currency,
            params.doc_clause.as_deref(),
        )?;

        let reuse_context = if use_parallel {
            None
        } else {
            Some(RefCell::new(base_context.clone()))
        };

        Ok(Self {
            params,
            cds_conventions,
            base_context,
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
            HazardBootstrapper::new(params.clone(), context.clone(), global_config.use_parallel)?;

        let mut prepared_quotes: Vec<CalibrationQuote> = Vec::with_capacity(cds_quotes.len());
        let mut curve_ids = HashMap::new();
        curve_ids.insert(
            "discount".to_string(),
            params.discount_curve_id.to_string(),
        );
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
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        };

        let mut new_context = context.clone();
        new_context.insert_mut(curve);
        Ok((new_context, report))
    }

    fn with_temp_context<F, T>(&self, curve: &HazardCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();
            ctx.insert_mut(curve.clone());
            // Sync CreditIndex if it exists (so pricer sees trial curve)
            if let Ok(idx) = ctx.credit_index_ref(&self.params.curve_id) {
                let mut updated = idx.clone();
                updated.index_credit_curve = std::sync::Arc::new(curve.clone());
                ctx.insert_credit_index_mut(&self.params.curve_id, updated);
            }
            op(&ctx)
        } else {
            let mut temp_context = self.base_context.clone();
            temp_context.insert_mut(curve.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::calibration::pricing::quote_factory; // Removed
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
        let target = HazardBootstrapper::new(base_params(), MarketContext::default(), false)
            .expect("target");
        let err = target
            .validate_knot(1.0, -1e-6)
            .expect_err("should reject negative hazard");
        assert!(err.to_string().to_lowercase().contains("negative hazard"));
    }

    #[test]
    fn validate_knot_rejects_hazard_above_max() {
        let target = HazardBootstrapper::new(base_params(), MarketContext::default(), false)
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
        let target = HazardBootstrapper::new(p, MarketContext::default(), false).expect("target");

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

    // Test removed (legacy types). Covered by parity tests.
}

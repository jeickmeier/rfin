//! Bootstrapper for base correlation curves built from tranche quotes.

use crate::calibration::api::schema::BaseCorrelationParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::prepared::{CalibrationQuote, CdsTrancheCalibrationQuote};
use crate::calibration::solver::bootstrap::SequentialBootstrapper;
use crate::calibration::solver::traits::BootstrapTarget;
use crate::calibration::CalibrationReport;
use crate::market::build::cds_tranche::{build_cds_tranche_instrument, CdsTrancheBuildOverrides};
use crate::market::build::context::BuildCtx;
use crate::market::build::prepared::PreparedQuote;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::cds_tranche::CdsTrancheQuote;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use finstack_core::dates::{DateExt, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;
use std::sync::Arc;

/// Bootstrapper that calibrates a [`BaseCorrelationCurve`] from tranche quotes.
pub struct BaseCorrelationBootstrapper {
    /// Calibration inputs (curve IDs, schedule conventions, detachment points).
    pub params: BaseCorrelationParams,
    /// Baseline market context used when pricing trial curves.
    pub base_context: MarketContext,
}

impl BaseCorrelationBootstrapper {
    /// Create a new base correlation bootstrapper.
    pub fn new(params: BaseCorrelationParams, base_context: MarketContext) -> Self {
        Self {
            params,
            base_context,
        }
    }

    fn normalize_pct(value: f64) -> f64 {
        if (0.0..=1.0).contains(&value) {
            value * 100.0
        } else {
            value
        }
    }

    fn validate_monotone_and_bounds(knots: &[(f64, f64)]) -> Result<()> {
        if knots.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }

        for (detachment, corr) in knots {
            if !detachment.is_finite() || *detachment <= 0.0 || *detachment > 100.0 {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ));
            }
            if !corr.is_finite() || *corr < 0.0 || *corr > 1.0 {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ));
            }
        }
        Ok(())
    }

    fn build_ctx(&self) -> BuildCtx {
        let mut curve_ids = HashMap::default();
        curve_ids.insert(
            "discount".to_string(),
            self.params.discount_curve_id.to_string(),
        );
        curve_ids.insert("credit".to_string(), self.params.index_id.clone());
        BuildCtx::new(self.params.base_date, self.params.notional, curve_ids)
    }

    fn prepare_quotes(&self, quotes: Vec<CdsTrancheQuote>) -> Result<Vec<CalibrationQuote>> {
        let mut prepared = Vec::with_capacity(quotes.len());
        let build_ctx = self.build_ctx();
        let overrides = CdsTrancheBuildOverrides {
            series: self.params.series,
            payment_frequency: self.params.payment_frequency,
            day_count: self.params.day_count,
            business_day_convention: self.params.business_day_convention,
            calendar_id: self.params.calendar_id.clone(),
            use_imm_dates: self.params.use_imm_dates,
        };
        if !self.params.detachment_points.is_empty() {
            for d in &self.params.detachment_points {
                if !d.is_finite() || *d <= 0.0 || *d > 100.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "detachment point {} must be in (0, 100]",
                        d
                    )));
                }
            }
        }
        let expected_detachments: Vec<f64> = self
            .params
            .detachment_points
            .iter()
            .map(|d| Self::normalize_pct(*d))
            .collect();
        let mut seen_detachments: Vec<f64> = Vec::new();
        let maturity_tol_days: i64 = if self.params.use_imm_dates { 40 } else { 7 };
        let time_dc = self
            .params
            .day_count
            .unwrap_or(finstack_core::dates::DayCount::Act365F);

        for q in quotes {
            let (index, attachment, detachment, maturity, upfront_pct, convention) = match &q {
                CdsTrancheQuote::CDSTranche {
                    index,
                    attachment,
                    detachment,
                    maturity,
                    upfront_pct,
                    convention,
                    ..
                } => (
                    index,
                    *attachment,
                    *detachment,
                    *maturity,
                    *upfront_pct,
                    convention,
                ),
            };

            if index != &self.params.index_id {
                return Err(finstack_core::Error::Validation(format!(
                    "Tranche quote index '{}' does not match params.index_id '{}'",
                    index, self.params.index_id
                )));
            }

            ConventionRegistry::try_global()?.require_cds(convention)?;
            // Build a pricer instrument with *no embedded upfront cashflow* so that
            // `tranche.upfront(...)` returns the model-implied upfront for the running coupon.
            let q_pricing = match &q {
                CdsTrancheQuote::CDSTranche {
                    id,
                    index,
                    attachment,
                    detachment,
                    maturity,
                    running_spread_bp,
                    convention,
                    ..
                } => CdsTrancheQuote::CDSTranche {
                    id: id.clone(),
                    index: index.clone(),
                    attachment: *attachment,
                    detachment: *detachment,
                    maturity: *maturity,
                    upfront_pct: 0.0,
                    running_spread_bp: *running_spread_bp,
                    convention: convention.clone(),
                },
            };

            let instrument = build_cds_tranche_instrument(&q_pricing, &build_ctx, &overrides)
                .map_err(|e| {
                    finstack_core::Error::Validation(format!(
                        "Failed to build tranche instrument: {e}"
                    ))
                })?;

            let detachment_pct = Self::normalize_pct(detachment);
            if !expected_detachments.is_empty()
                && !expected_detachments
                    .iter()
                    .any(|d| (d - detachment_pct).abs() <= 1e-8)
            {
                return Err(finstack_core::Error::Validation(format!(
                    "Tranche detachment {} not in params.detachment_points {:?}",
                    detachment_pct, expected_detachments
                )));
            }
            seen_detachments.push(detachment_pct);

            if self.params.maturity_years > 0.0 {
                // `maturity_years` is a *tenor-like* input (e.g. 5Y), not a day-count year
                // fraction. Comparing via `year_fraction` breaks for conventions like ACT/360.
                let months = (self.params.maturity_years * 12.0).round() as i32;
                let expected = self.params.base_date.add_months(months);
                let diff_days = (maturity - expected).whole_days().abs();
                if diff_days > maturity_tol_days {
                    return Err(finstack_core::Error::Validation(format!(
                        "Tranche maturity {} differs from base_date+{}M={} by {} days (tol {} days)",
                        maturity, months, expected, diff_days, maturity_tol_days
                    )));
                }
            }
            let pillar_date = maturity;
            let pillar_time =
                time_dc.year_fraction(self.params.base_date, maturity, DayCountCtx::default())?;

            let prepared_quote = PreparedQuote::new(
                Arc::new(q.clone()),
                Arc::<dyn crate::instruments::common::traits::Instrument>::from(instrument),
                pillar_date,
                pillar_time,
            );

            // Market tranche upfront is quoted as a percentage of tranche notional.
            // Normalize attachment/detachment to percent (0-100) for consistent handling,
            // then compute width as a fraction (0-1) for the tranche notional calculation.
            let attachment_pct = Self::normalize_pct(attachment);
            let detachment_pct_local = Self::normalize_pct(detachment);
            let width_frac = ((detachment_pct_local - attachment_pct) / 100.0).max(0.0);
            let tranche_notional = self.params.notional * width_frac;
            let upfront_money = Some(Money::new(
                upfront_pct * 0.01 * tranche_notional,
                self.params.currency,
            ));
            prepared.push(CalibrationQuote::CdsTranche(CdsTrancheCalibrationQuote {
                prepared: prepared_quote,
                upfront: upfront_money,
                detachment_pct,
            }));
        }

        if !expected_detachments.is_empty() {
            for expected in expected_detachments {
                if !seen_detachments
                    .iter()
                    .any(|d| (d - expected).abs() <= 1e-8)
                {
                    return Err(finstack_core::Error::Validation(format!(
                        "Missing tranche detachment {} from quotes",
                        expected
                    )));
                }
            }
        }

        Ok(prepared)
    }

    /// Prepare quotes and run the sequential bootstrap for base correlation.
    pub fn solve(
        params: &BaseCorrelationParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let tranche_quotes: Vec<CdsTrancheQuote> = quotes.extract_quotes();
        if tranche_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let target = BaseCorrelationBootstrapper::new(params.clone(), context.clone());
        let prepared_quotes = target.prepare_quotes(tranche_quotes)?;

        // Base correlation uses discount curve validation tolerance as the closest target-specific config.
        // Could add a dedicated base_correlation_curve config in the future if needed.
        let success_tolerance = Some(global_config.discount_curve.validation_tolerance);

        let (curve, mut report) = SequentialBootstrapper::bootstrap(
            &target,
            &prepared_quotes,
            Vec::new(),
            global_config,
            success_tolerance,
            None,
        )?;

        report.update_solver_config(global_config.solver.clone());

        let mut new_context = context.clone().insert_base_correlation(curve.clone());
        if let Ok(idx) = new_context.credit_index(params.index_id.as_str()) {
            let mut updated = idx.as_ref().clone();
            updated.base_correlation_curve = Arc::new(curve.clone());
            new_context = new_context.insert_credit_index(params.index_id.as_str(), updated);
        }

        Ok((new_context, report))
    }
}

impl BootstrapTarget for BaseCorrelationBootstrapper {
    type Quote = CalibrationQuote;
    type Curve = BaseCorrelationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        match quote {
            CalibrationQuote::CdsTranche(pq) => Ok(pq.detachment_pct),
            _ => Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            )),
        }
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        let mut sorted_knots = knots.to_vec();
        if sorted_knots.iter().any(|(d, _)| !d.is_finite()) {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }

        if sorted_knots.len() == 1 {
            let (k, v) = sorted_knots[0];
            let bump = 10.0;
            let k2 = if k + bump <= 100.0 {
                k + bump
            } else if k >= bump {
                k - bump
            } else {
                (k + 1.0).min(100.0)
            };
            if (k2 - k).abs() > 1e-12 {
                sorted_knots.push((k2, v));
            }
        }

        sorted_knots.sort_by(|a, b| a.0.total_cmp(&b.0));
        sorted_knots.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-12);

        if sorted_knots.len() < 2 {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        Self::validate_monotone_and_bounds(&sorted_knots)?;

        BaseCorrelationCurve::builder(format!("{}_CORR", self.params.index_id))
            .knots(sorted_knots)
            .build()
    }

    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        let curve = self.build_curve(knots)?;
        let validation = curve.validate_arbitrage_free();
        if !validation.is_arbitrage_free {
            return Err(finstack_core::Error::Validation(format!(
                "Base correlation curve is not arbitrage-free: {:?}",
                validation.violations
            )));
        }
        Ok(curve)
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let (pq, upfront) = match quote {
            CalibrationQuote::CdsTranche(pq) => (&pq.prepared, &pq.upfront),
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::InputError::Invalid,
                ))
            }
        };

        let mut temp_context = self
            .base_context
            .clone()
            .insert_base_correlation(curve.clone());
        if let Ok(idx) = temp_context.credit_index(self.params.index_id.as_str()) {
            let mut updated = idx.as_ref().clone();
            updated.base_correlation_curve = Arc::new(curve.clone());
            temp_context = temp_context.insert_credit_index(self.params.index_id.as_str(), updated);
        }

        let tranche = pq
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cds_tranche::CdsTranche>()
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Base correlation calibration requires a CdsTranche instrument".to_string(),
                )
            })?;

        // Fit to the market upfront quote directly (vendor-style).
        let model_upfront = tranche.upfront(&temp_context, self.params.base_date)?;
        let market_upfront = upfront.as_ref().map(|m| m.amount()).unwrap_or(0.0);
        Ok((model_upfront - market_upfront) / self.params.notional)
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        // Return the previous knot's correlation as both the starting point and the
        // lower bound for the monotonicity constraint (β(K₂) ≥ β(K₁) for K₂ > K₁).
        // For the first quote, return 0.0 (no constraint from previous knots).
        let prev = previous_knots.last().map(|(_, v)| *v).unwrap_or(0.0);
        Ok(prev.clamp(0.0, 0.999))
    }

    fn scan_points(&self, _quote: &Self::Quote, initial_guess: f64) -> Result<Vec<f64>> {
        // Base correlation is a bounded parameter in [0, 1) with a monotonicity
        // constraint: β(K₂) ≥ β(K₁) for K₂ > K₁. The `initial_guess` provides the
        // previous knot's correlation, which is the lower bound for valid solutions.
        //
        // Generate a dense bounded scan grid in [low, hi] so the solver only
        // explores the feasible region, avoiding validation errors from
        // non-monotonic trial curves.
        let mut pts = Vec::with_capacity(64);
        let hi = 0.999_f64;

        // Lower bound from monotonicity: new correlation >= previous correlation.
        let low = initial_guess.clamp(0.0, hi);

        pts.push(low);
        pts.push(hi);

        // Linear grid across the feasible region [low, hi].
        const N: usize = 48;
        for i in 0..=N {
            let x = low + (i as f64) / (N as f64) * (hi - low);
            pts.push(x);
        }

        // Extra refinement around a central estimate.
        // Start searching from a point in the interior of the feasible region.
        let center = if low < 0.5 {
            (low + 0.30).min(0.85)
        } else {
            (low + hi) / 2.0
        };
        for dx in [1e-4, 5e-4, 1e-3, 5e-3, 1e-2, 0.05] {
            for s in [-1.0, 1.0] {
                let x = (center + s * dx).clamp(low, hi);
                pts.push(x);
            }
        }

        pts.retain(|x| x.is_finite());
        pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        Ok(pts)
    }

    fn validate_knot(&self, _time: f64, value: f64) -> Result<()> {
        if !value.is_finite() || !(0.0..=0.999).contains(&value) {
            return Err(finstack_core::Error::Validation(format!(
                "Base correlation must be in [0, 0.999], got {}",
                value
            )));
        }
        Ok(())
    }
}

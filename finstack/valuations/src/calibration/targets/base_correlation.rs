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
use finstack_core::dates::{Date, DateExt, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;
use std::sync::Arc;

// =============================================================================
// Helper Types and Functions
// =============================================================================

/// Extracted fields from a CDS tranche quote for validation.
struct TrancheQuoteFields<'a> {
    index: &'a str,
    attachment: f64,
    detachment: f64,
    maturity: Date,
    upfront_pct: f64,
    convention: &'a crate::market::conventions::ids::CdsConventionKey,
}

impl<'a> TrancheQuoteFields<'a> {
    fn extract(quote: &'a CdsTrancheQuote) -> Self {
        match quote {
            CdsTrancheQuote::CDSTranche {
                index,
                attachment,
                detachment,
                maturity,
                upfront_pct,
                convention,
                ..
            } => Self {
                index: index.as_str(),
                attachment: *attachment,
                detachment: *detachment,
                maturity: *maturity,
                upfront_pct: *upfront_pct,
                convention,
            },
        }
    }
}

/// Validate that all detachment points in params are valid.
fn validate_detachment_points(points: &[f64]) -> Result<()> {
    for d in points {
        if !d.is_finite() || *d <= 0.0 || *d > 100.0 {
            return Err(finstack_core::Error::Validation(format!(
                "detachment point {d} must be in (0, 100]"
            )));
        }
    }
    Ok(())
}

/// Validate that a quote's index matches the expected index.
fn validate_quote_index(quote_index: &str, expected_index: &str) -> Result<()> {
    if quote_index != expected_index {
        return Err(finstack_core::Error::Validation(format!(
            "Tranche quote index '{quote_index}' does not match params.index_id '{expected_index}'"
        )));
    }
    Ok(())
}

/// Validate that a detachment point is in the expected set (if non-empty).
fn validate_detachment_in_expected(detachment_pct: f64, expected: &[f64]) -> Result<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let found = expected.iter().any(|d| (d - detachment_pct).abs() <= 1e-8);
    if !found {
        return Err(finstack_core::Error::Validation(format!(
            "Tranche detachment {detachment_pct} not in params.detachment_points {expected:?}"
        )));
    }
    Ok(())
}

/// Validate that a quote's maturity is within tolerance of the expected maturity.
fn validate_maturity_tolerance(
    maturity: Date,
    base_date: Date,
    maturity_years: f64,
    tol_days: i64,
) -> Result<()> {
    if maturity_years <= 0.0 {
        return Ok(());
    }
    // `maturity_years` is a *tenor-like* input (e.g. 5Y), not a day-count year
    // fraction. Comparing via `year_fraction` breaks for conventions like ACT/360.
    let months = (maturity_years * 12.0).round() as i32;
    let expected = base_date.add_months(months);
    let diff_days = (maturity - expected).whole_days().abs();
    if diff_days > tol_days {
        return Err(finstack_core::Error::Validation(format!(
            "Tranche maturity {maturity} differs from base_date+{months}M={expected} by {diff_days} days (tol {tol_days} days)"
        )));
    }
    Ok(())
}

/// Validate that all expected detachments were seen in the quotes.
fn validate_all_detachments_seen(expected: &[f64], seen: &[f64]) -> Result<()> {
    for exp in expected {
        if !seen.iter().any(|d| (d - exp).abs() <= 1e-8) {
            return Err(finstack_core::Error::Validation(format!(
                "Missing tranche detachment {exp} from quotes"
            )));
        }
    }
    Ok(())
}

/// Create a pricing quote with zero upfront (for model-implied upfront calculation).
fn create_pricing_quote(quote: &CdsTrancheQuote) -> CdsTrancheQuote {
    match quote {
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
    }
}

/// Compute the upfront money amount from quote fields.
fn compute_upfront_money(
    attachment: f64,
    detachment: f64,
    upfront_pct: f64,
    notional: f64,
    currency: finstack_core::currency::Currency,
) -> Money {
    let attachment_pct = normalize_pct(attachment);
    let detachment_pct = normalize_pct(detachment);
    let width_frac = ((detachment_pct - attachment_pct) / 100.0).max(0.0);
    let tranche_notional = notional * width_frac;
    Money::new(upfront_pct * 0.01 * tranche_notional, currency)
}

/// Normalize a value to percentage (0-100 scale).
fn normalize_pct(value: f64) -> f64 {
    if (0.0..=1.0).contains(&value) {
        value * 100.0
    } else {
        value
    }
}

// =============================================================================
// Main Bootstrapper
// =============================================================================

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

    fn build_overrides(&self) -> CdsTrancheBuildOverrides {
        CdsTrancheBuildOverrides {
            series: self.params.series,
            payment_frequency: self.params.payment_frequency,
            day_count: self.params.day_count,
            business_day_convention: self.params.business_day_convention,
            calendar_id: self.params.calendar_id.clone(),
            use_imm_dates: self.params.use_imm_dates,
        }
    }

    fn normalized_expected_detachments(&self) -> Vec<f64> {
        self.params
            .detachment_points
            .iter()
            .map(|d| normalize_pct(*d))
            .collect()
    }

    /// Build a single calibration quote from a tranche quote.
    fn build_calibration_quote(
        &self,
        quote: &CdsTrancheQuote,
        build_ctx: &BuildCtx,
        overrides: &CdsTrancheBuildOverrides,
        time_dc: DayCount,
    ) -> Result<CalibrationQuote> {
        let fields = TrancheQuoteFields::extract(quote);

        // Build pricing instrument without embedded upfront
        let pricing_quote = create_pricing_quote(quote);
        let instrument = build_cds_tranche_instrument(&pricing_quote, build_ctx, overrides)
            .map_err(|e| {
                finstack_core::Error::Validation(format!("Failed to build tranche instrument: {e}"))
            })?;

        let pillar_time = time_dc.year_fraction(
            self.params.base_date,
            fields.maturity,
            DayCountCtx::default(),
        )?;

        let prepared_quote = PreparedQuote::new(
            Arc::new(quote.clone()),
            Arc::<dyn crate::instruments::common::traits::Instrument>::from(instrument),
            fields.maturity,
            pillar_time,
        );

        let detachment_pct = normalize_pct(fields.detachment);
        let upfront_money = compute_upfront_money(
            fields.attachment,
            fields.detachment,
            fields.upfront_pct,
            self.params.notional,
            self.params.currency,
        );

        Ok(CalibrationQuote::CdsTranche(CdsTrancheCalibrationQuote {
            prepared: prepared_quote,
            upfront: Some(upfront_money),
            detachment_pct,
        }))
    }

    fn prepare_quotes(&self, quotes: Vec<CdsTrancheQuote>) -> Result<Vec<CalibrationQuote>> {
        // Validate params
        if !self.params.detachment_points.is_empty() {
            validate_detachment_points(&self.params.detachment_points)?;
        }

        let expected_detachments = self.normalized_expected_detachments();
        let maturity_tol_days: i64 = if self.params.use_imm_dates { 40 } else { 7 };
        let time_dc = self.params.day_count.unwrap_or(DayCount::Act365F);
        let build_ctx = self.build_ctx();
        let overrides = self.build_overrides();

        let mut prepared = Vec::with_capacity(quotes.len());
        let mut seen_detachments = Vec::new();

        for q in quotes {
            let fields = TrancheQuoteFields::extract(&q);

            // Validate quote
            validate_quote_index(fields.index, &self.params.index_id)?;
            ConventionRegistry::try_global()?.require_cds(fields.convention)?;

            let detachment_pct = normalize_pct(fields.detachment);
            validate_detachment_in_expected(detachment_pct, &expected_detachments)?;
            validate_maturity_tolerance(
                fields.maturity,
                self.params.base_date,
                self.params.maturity_years,
                maturity_tol_days,
            )?;

            // Build calibration quote
            let calib_quote = self.build_calibration_quote(&q, &build_ctx, &overrides, time_dc)?;
            seen_detachments.push(detachment_pct);
            prepared.push(calib_quote);
        }

        // Validate all expected detachments were seen
        if !expected_detachments.is_empty() {
            validate_all_detachments_seen(&expected_detachments, &seen_detachments)?;
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

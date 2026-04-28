//! Cross-currency basis curve bootstrap target.
//!
//! Derives a foreign currency discount curve from a domestic OIS curve,
//! FX spot rate, and FX forward or XCCY basis swap quotes. Optionally
//! produces a [`BasisSpreadCurve`] as a byproduct.

use crate::calibration::api::schema::XccyBasisParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::prepared::CalibrationQuote;
use crate::calibration::solver::bootstrap::SequentialBootstrapper;
use crate::calibration::solver::traits::BootstrapTarget;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{BasisSpreadCurve, DiscountCurve};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::sync::Arc;

/// Parameters for constructing an [`XccyBasisTarget`].
#[derive(Clone)]
pub(crate) struct XccyBasisTargetParams {
    /// Base date for the calibration.
    pub(crate) base_date: Date,
    /// ID for the foreign discount curve being built.
    pub(crate) curve_id: CurveId,
    /// Pre-calibrated domestic discount curve.
    pub(crate) domestic_discount: Arc<DiscountCurve>,
    /// Interpolation style for the foreign curve.
    pub(crate) solve_interp: InterpStyle,
    /// Extrapolation policy for the foreign curve.
    pub(crate) extrapolation: ExtrapolationPolicy,
    /// Base market context for pricing XCCY instruments.
    pub(crate) base_context: MarketContext,
}

/// Bootstrap target for cross-currency basis curve calibration.
///
/// Solves for the foreign discount curve knots that reprice FX forwards
/// or XCCY basis swaps given a known domestic discount curve and FX spot.
pub(crate) struct XccyBasisTarget {
    params: XccyBasisTargetParams,
}

impl XccyBasisTarget {
    /// Create a new cross-currency basis target.
    pub(crate) fn new(params: XccyBasisTargetParams) -> Self {
        Self { params }
    }

    /// Execute the full calibration for a cross-currency basis step.
    pub(crate) fn solve(
        schema_params: &XccyBasisParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        let domestic_discount = context.get_discount(&schema_params.domestic_discount_id)?;

        let mut config = global_config.clone();
        config.calibration_method = schema_params.method.clone();

        let rates_quotes: Vec<RateQuote> = quotes.extract_quotes();
        if rates_quotes.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        let curve_dc = schema_params
            .conventions
            .curve_day_count
            .unwrap_or(finstack_core::dates::DayCount::Act365F);

        let mut curve_ids = finstack_core::HashMap::default();
        curve_ids.insert("discount".to_string(), schema_params.curve_id.to_string());

        let build_ctx = crate::market::build::context::BuildCtx::new(
            schema_params.base_date,
            1_000_000.0,
            curve_ids,
        );

        let mut prepared_quotes: Vec<CalibrationQuote> = Vec::with_capacity(rates_quotes.len());
        for q in rates_quotes {
            let prepared = crate::market::build::prepared::prepare_rate_quote(
                q,
                &build_ctx,
                curve_dc,
                schema_params.base_date,
                true,
            )?;
            prepared_quotes.push(CalibrationQuote::Rates(prepared));
        }

        let target = Self::new(XccyBasisTargetParams {
            base_date: schema_params.base_date,
            curve_id: schema_params.curve_id.clone(),
            domestic_discount: Arc::clone(&domestic_discount),
            solve_interp: schema_params.interpolation,
            extrapolation: schema_params.extrapolation,
            base_context: context.clone(),
        });

        let success_tolerance = Some(config.discount_curve.validation_tolerance);

        let (curve, report) = SequentialBootstrapper::bootstrap(
            &target,
            &prepared_quotes,
            vec![(0.0, 1.0)],
            &config,
            success_tolerance,
            None,
        )?;

        let mut new_context = context.clone().insert(curve.clone());

        // Extract basis spread curve as byproduct if requested.
        if let Some(spread_id) = &schema_params.basis_spread_curve_id {
            let knots = curve.knots();
            let foreign_dfs = curve.dfs();

            let mut spread_knots = Vec::with_capacity(knots.len());
            for (i, &t) in knots.iter().enumerate() {
                if t <= 0.0 {
                    spread_knots.push((t, 0.0));
                    continue;
                }
                let z_foreign = -(foreign_dfs[i].ln()) / t;
                let z_domestic = -(domestic_discount.df(t).ln()) / t;
                spread_knots.push((t, z_foreign - z_domestic));
            }

            if let Ok(spread_curve) = BasisSpreadCurve::builder(spread_id.clone())
                .base_date(schema_params.base_date)
                .knots(spread_knots)
                .interp(schema_params.interpolation)
                .extrapolation(schema_params.extrapolation)
                .build()
            {
                new_context = new_context.insert(spread_curve);
            }
        }

        Ok((new_context, report))
    }
}

impl BootstrapTarget for XccyBasisTarget {
    type Quote = CalibrationQuote;
    type Curve = DiscountCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        Ok(quote.pillar_time())
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        DiscountCurve::builder(self.params.curve_id.clone())
            .base_date(self.params.base_date)
            .knots(knots.to_vec())
            .interp(self.params.solve_interp)
            .extrapolation(self.params.extrapolation)
            .build()
    }

    fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        DiscountCurve::builder(self.params.curve_id.clone())
            .base_date(self.params.base_date)
            .knots(knots.to_vec())
            .interp(self.params.solve_interp)
            .extrapolation(self.params.extrapolation)
            .allow_non_monotonic()
            .build_for_solver()
    }

    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        DiscountCurve::builder(self.params.curve_id.clone())
            .base_date(self.params.base_date)
            .knots(knots.to_vec())
            .interp(self.params.solve_interp)
            .extrapolation(self.params.extrapolation)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let ctx = self.params.base_context.clone().insert(curve.clone());
        let pv = quote
            .get_instrument()
            .value_raw(&ctx, self.params.base_date)?;
        Ok(pv)
    }

    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let t = self.quote_time(quote)?;
        if let Some(&(prev_t, prev_df)) = previous_knots.last() {
            // Geometric extrapolation from last known knot
            if prev_t > 0.0 && t > prev_t {
                let rate = -prev_df.ln() / prev_t;
                return Ok((-rate * t).exp());
            }
            Ok(prev_df)
        } else {
            Ok(self.params.domestic_discount.df(t))
        }
    }

    fn validate_knot(&self, _time: f64, value: f64) -> Result<()> {
        if value <= 0.0 || value > 1.5 {
            return Err(finstack_core::Error::Validation(format!(
                "Foreign discount factor out of range: {value:.8}"
            )));
        }
        Ok(())
    }
}

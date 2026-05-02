//! CDS-specific DV01 calculator.
//!
//! CDS rate risk is a cross-curve sensitivity when the credit curve is stored
//! with the market par spreads used to build it: after a rate-curve bump, the
//! hazard curve must be re-bootstrapped from unchanged CDS spreads. This matches
//! Bloomberg-style IR DV01 for CDS screens.

use super::{hazard_with_deal_quote, market_doc_clause};
use crate::calibration::api::schema::DiscountCurveParams;
use crate::calibration::bumps::hazard::bump_hazard_spreads_with_doc_clause;
use crate::calibration::bumps::rates::bump_discount_curve;
use crate::calibration::bumps::BumpRequest;
use crate::calibration::CalibrationMethod;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::market::quotes::ids::{Pillar, QuoteId};
use crate::market::quotes::rates::RateQuote;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::market_data::term_structures::{
    DiscountCurve, DiscountCurveRateCalibration, DiscountCurveRateQuoteType,
};
use finstack_core::Result;
use std::sync::Arc;
use time::Duration;

const MIN_BUMP_BP: f64 = 1e-10;

/// CDS DV01 calculator with par-spread hazard re-bootstrap when possible.
pub(crate) struct CdsDv01Calculator;

impl CdsDv01Calculator {
    fn discount_curve_from_rate_quote_bump(
        curve: &DiscountCurve,
        calibration: &DiscountCurveRateCalibration,
        context: &MarketContext,
        bump_bp: f64,
    ) -> Result<DiscountCurve> {
        let index = finstack_core::types::IndexId::new(calibration.index_id.as_str());
        let mut quotes = Vec::with_capacity(calibration.quotes.len());
        for quote in &calibration.quotes {
            let pillar = Pillar::Tenor(quote.tenor.parse()?);
            let id = QuoteId::new(format!("{}-{}", curve.id(), quote.tenor));
            let rate_quote = match quote.quote_type {
                DiscountCurveRateQuoteType::Deposit => RateQuote::Deposit {
                    id,
                    index: index.clone(),
                    pillar,
                    rate: quote.rate,
                },
                DiscountCurveRateQuoteType::Swap => RateQuote::Swap {
                    id,
                    index: index.clone(),
                    pillar,
                    rate: quote.rate,
                    spread_decimal: None,
                },
            };
            quotes.push(rate_quote);
        }

        let first_rate = calibration
            .quotes
            .first()
            .map(|quote| quote.rate)
            .unwrap_or(0.0);
        let fixings = ScalarTimeSeries::new(
            format!("FIXING:{}", curve.id()),
            vec![
                (curve.base_date() - Duration::days(3), first_rate),
                (curve.base_date() - Duration::days(2), first_rate),
                (curve.base_date() - Duration::days(1), first_rate),
                (curve.base_date(), first_rate),
            ],
            None,
        )?;
        let base_context = context.clone().insert_series(fixings);

        let params = DiscountCurveParams {
            curve_id: curve.id().clone(),
            currency: calibration.currency,
            base_date: curve.base_date(),
            method: CalibrationMethod::Bootstrap,
            interpolation: curve.interp_style(),
            extrapolation: curve.extrapolation(),
            pricing_discount_id: None,
            pricing_forward_id: None,
            conventions: crate::calibration::RatesStepConventions {
                curve_day_count: Some(curve.day_count()),
            },
        };

        bump_discount_curve(
            &quotes,
            &params,
            &base_context,
            &BumpRequest::Parallel(bump_bp),
        )
    }

    fn price_at_rate_bump(
        cds: &CreditDefaultSwap,
        context: &MetricContext,
        bump_bp: f64,
        rebootstrap_hazard: bool,
    ) -> Result<f64> {
        let mut bumped_market: MarketContext = context.curves.as_ref().clone();
        let base_discount = context
            .curves
            .get_discount(cds.premium.discount_curve_id.as_str())?;
        if let Some(calibration) = base_discount.rate_calibration() {
            let bumped_discount = Self::discount_curve_from_rate_quote_bump(
                base_discount.as_ref(),
                calibration,
                context.curves.as_ref(),
                bump_bp,
            )?;
            bumped_market = bumped_market.insert(bumped_discount);
        } else {
            bumped_market.apply_curve_bump_in_place(
                &cds.premium.discount_curve_id,
                BumpSpec::parallel_bp(bump_bp),
            )?;
        }

        if rebootstrap_hazard {
            let base_hazard = context
                .curves
                .get_hazard(cds.protection.credit_curve_id.as_str())?;
            let recalibrated = bump_hazard_spreads_with_doc_clause(
                base_hazard.as_ref(),
                &bumped_market,
                &BumpRequest::Parallel(0.0),
                Some(&cds.premium.discount_curve_id),
                Some(market_doc_clause(cds)),
            )?;
            bumped_market = bumped_market.insert(recalibrated);
        }

        context.reprice_raw(&bumped_market, context.as_of)
    }
}

impl MetricCalculator for CdsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: CreditDefaultSwap = context.instrument_as::<CreditDefaultSwap>()?.clone();
        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let bump_bp = defaults.rate_bump_bp;
        if bump_bp.abs() <= MIN_BUMP_BP {
            return Ok(0.0);
        }

        let original_curves = Arc::clone(&context.curves);
        let result = (|| {
            let hazard = original_curves.get_hazard(cds.protection.credit_curve_id.as_str())?;
            if let Some(quote_hazard) = hazard_with_deal_quote(&cds, hazard.as_ref())? {
                context.curves = Arc::new(original_curves.as_ref().clone().insert(quote_hazard));
            }

            let hazard = context
                .curves
                .get_hazard(cds.protection.credit_curve_id.as_str())?;
            let rebootstrap_hazard = hazard.par_spread_points().next().is_some();

            let pv_up = Self::price_at_rate_bump(&cds, context, bump_bp, rebootstrap_hazard)?;
            let pv_down = Self::price_at_rate_bump(&cds, context, -bump_bp, rebootstrap_hazard)?;

            Ok((pv_up - pv_down) / (2.0 * bump_bp))
        })();
        context.curves = original_curves;
        result
    }
}

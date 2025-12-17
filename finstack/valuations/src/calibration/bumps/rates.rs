//! Shared rates curve bumping logic (v2 plan-driven calibration).

use super::BumpRequest;
use crate::calibration::adapters::handlers::execute_step;
use crate::calibration::api::schema::{DiscountCurveParams, StepParams};
use crate::calibration::config::CalibrationMethod;
use crate::calibration::pricing::RatesStepConventions;
use crate::calibration::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use crate::calibration::CalibrationConfig;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::types::Currency;

/// Bump a discount curve by shocking rates quotes and re-calibrating via the v2 step engine.
pub fn bump_discount_curve(
    quotes: &[RatesQuote],
    params: &DiscountCurveParams,
    base_context: &MarketContext,
    bump: &BumpRequest,
) -> finstack_core::Result<DiscountCurve> {
    let as_of = params.base_date;

    // Clone quotes to apply bumps.
    let mut bumped_quotes: Vec<RatesQuote> = quotes.to_vec();

    match bump {
        BumpRequest::Parallel(bp) => {
            let bump_decimal = bp * 1e-4;
            bumped_quotes = bumped_quotes
                .into_iter()
                .map(|q| q.bump_rate_decimal(bump_decimal))
                .collect();
        }
        BumpRequest::Tenors(targets) => {
            for (target_t, bp) in targets {
                if let Some(idx) = find_closest_quote(&bumped_quotes, *target_t, as_of) {
                    let bump_decimal = bp * 1e-4;
                    bumped_quotes[idx] = bumped_quotes[idx].bump_rate_decimal(bump_decimal);
                }
            }
        }
    }

    let market_quotes: Vec<MarketQuote> =
        bumped_quotes.into_iter().map(MarketQuote::Rates).collect();
    let step = StepParams::Discount(params.clone());
    let cfg = CalibrationConfig::default();
    let (ctx, _report) = execute_step(&step, &market_quotes, base_context, &cfg)?;

    Ok(ctx.get_discount_ref(params.curve_id.as_str())?.clone())
}

/// Find the quote closest to the target maturity.
pub fn find_closest_quote(quotes: &[RatesQuote], target_years: f64, as_of: Date) -> Option<usize> {
    let dc = DayCount::Act365F; // Simple day count for proximity check
    quotes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let a_yf = dc
                .year_fraction(as_of, a.maturity_date(), DayCountCtx::default())
                .unwrap_or(0.0);
            let b_yf = dc
                .year_fraction(as_of, b.maturity_date(), DayCountCtx::default())
                .unwrap_or(0.0);
            let a_dist = (a_yf - target_years).abs();
            let b_dist = (b_yf - target_years).abs();
            a_dist
                .partial_cmp(&b_dist)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

/// Bump discount curve by synthesizing par instruments (OIS Swaps) from the curve, shocking them, and re-calibrating.
pub fn bump_discount_curve_synthetic(
    curve: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    as_of: Date,
) -> finstack_core::Result<DiscountCurve> {
    let curve_id = curve.id();
    let base_date = as_of;
    let knots = curve.knots();

    // Determine currency from ID or default
    let id_str = curve_id.as_str();
    let currency = if id_str.contains("USD") {
        Currency::USD
    } else if id_str.contains("EUR") {
        Currency::EUR
    } else if id_str.contains("GBP") {
        Currency::GBP
    } else if id_str.contains("JPY") {
        Currency::JPY
    } else {
        Currency::USD
    };

    // Synthesize deposit-style quotes for each knot (excluding t≈0) and re-calibrate.

    let mut quotes = Vec::new();
    let dc = DayCount::Act365F;
    let dc_ctx = DayCountCtx::default();

    for &t in knots {
        if t <= 0.0001 {
            continue;
        } // Skip t=0 or very small

        let df = curve.df(t);
        let maturity_days = (t * 365.25).round() as i64;
        let maturity = base_date + time::Duration::days(maturity_days);

        let yf = dc.year_fraction(base_date, maturity, dc_ctx).unwrap_or(t);

        // Simple rate: DF = 1 / (1 + r * t)  => 1 + r*t = 1/DF => r*t = 1/DF - 1 => r = (1/DF - 1)/t
        let rate = if yf > 1e-4 {
            (1.0 / df - 1.0) / yf
        } else {
            0.0
        };

        quotes.push(RatesQuote::Deposit {
            maturity,
            rate,
            conventions: InstrumentConventions::default()
                .with_day_count(dc)
                .with_settlement_days(0),
        });
    }

    let params = DiscountCurveParams {
        curve_id: curve_id.clone(),
        currency,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        extrapolation: ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: RatesStepConventions {
            curve_day_count: Some(DayCount::Act365F),
            settlement_days: Some(0),
            use_settlement_start: Some(false),
            ..Default::default()
        },
    };

    bump_discount_curve(&quotes, &params, context, bump)
}

//! Shared rates curve bumping logic (plan-driven calibration).

use super::BumpRequest;
use crate::calibration::api::schema::{DiscountCurveParams, StepParams};
use crate::calibration::config::CalibrationMethod;
use crate::calibration::config::RatesStepConventions;
use crate::calibration::step_runtime;
use crate::calibration::CalibrationConfig;
use crate::market::quotes::ids::{Pillar, QuoteId};
use crate::market::quotes::market_quote::MarketQuote;
use crate::market::quotes::rates::RateQuote;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::ExtrapolationPolicy;

/// Bump a discount curve by shocking rate quotes and re-calibrating.
///
/// This applies a [`BumpRequest`] to a collection of [`RateQuote`]s and
/// re-executes the calibration step to produce a new [`DiscountCurve`].
pub fn bump_discount_curve(
    quotes: &[RateQuote],
    params: &DiscountCurveParams,
    base_context: &MarketContext,
    bump: &BumpRequest,
) -> finstack_core::Result<DiscountCurve> {
    let as_of = params.base_date;

    // Clone quotes to apply bumps.
    let mut bumped_quotes: Vec<RateQuote> = quotes.to_vec();

    match bump {
        BumpRequest::Parallel(bp) => {
            bumped_quotes = bumped_quotes
                .into_iter()
                .map(|q| q.bump_rate_bp(*bp))
                .collect();
        }
        BumpRequest::Tenors(targets) => {
            for (target_t, bp) in targets {
                if let Some(idx) = find_closest_quote(&bumped_quotes, *target_t, as_of) {
                    bumped_quotes[idx] = bumped_quotes[idx].bump_rate_bp(*bp);
                }
            }
        }
    }

    let market_quotes: Vec<MarketQuote> =
        bumped_quotes.into_iter().map(MarketQuote::Rates).collect();
    let step = StepParams::Discount(params.clone());
    let cfg = CalibrationConfig::default();
    let (ctx, _report) =
        step_runtime::execute_params_and_apply(&step, &market_quotes, base_context, &cfg)?;

    Ok(ctx.get_discount(params.curve_id.as_str())?.as_ref().clone())
}

/// Helper to resolve maturity date of a quote.
fn resolve_maturity(q: &RateQuote, base_date: Date) -> Option<Date> {
    // Basic resolution using base_date + pillar
    // This ignores spot lag or BDC, but is sufficient for "closest quote" heuristics.
    match q {
        RateQuote::Deposit { pillar, .. } => resolve_pillar(pillar, base_date),
        RateQuote::Fra { end, .. } => resolve_pillar(end, base_date),
        RateQuote::Futures { expiry, .. } => Some(*expiry),
        RateQuote::Swap { pillar, .. } => resolve_pillar(pillar, base_date),
    }
}

fn resolve_pillar(pillar: &Pillar, base_date: Date) -> Option<Date> {
    match pillar {
        Pillar::Date(d) => Some(*d),
        Pillar::Tenor(t) => {
            // Approx add tenor
            // For bumping grouping, exact BDC usually doesn't change the "closest" logic significantly.
            t.add_to_date(
                base_date,
                None,
                finstack_core::dates::BusinessDayConvention::Following,
            )
            .ok()
        }
    }
}

/// Find the quote closest to the target maturity.
pub fn find_closest_quote(quotes: &[RateQuote], target_years: f64, as_of: Date) -> Option<usize> {
    let dc = DayCount::Act365F; // Simple day count for proximity check
    quotes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let a_date = resolve_maturity(a, as_of).unwrap_or(as_of);
            let b_date = resolve_maturity(b, as_of).unwrap_or(as_of);

            let a_yf = dc
                .year_fraction(as_of, a_date, DayCountCtx::default())
                .unwrap_or(0.0);
            let b_yf = dc
                .year_fraction(as_of, b_date, DayCountCtx::default())
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
///
/// Used when original quotes are unavailable. It implies par rates from
/// the current curve discount factors, applies shocks, and re-bootstraps.
pub fn bump_discount_curve_synthetic(
    curve: &finstack_core::market_data::term_structures::DiscountCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    as_of: Date,
    currency_override: Option<Currency>,
) -> finstack_core::Result<DiscountCurve> {
    let curve_id = curve.id();
    let base_date = as_of;
    let knots = curve.knots();

    // Determine currency: use explicit override if provided, otherwise fall back to string heuristic
    let currency = if let Some(ccy) = currency_override {
        ccy
    } else {
        tracing::warn!(
            "bump_discount_curve_synthetic: no currency provided for '{}', \
             falling back to string heuristic",
            curve_id.as_str()
        );
        let id_str = curve_id.as_str();
        if id_str.contains("USD") {
            Currency::USD
        } else if id_str.contains("EUR") {
            Currency::EUR
        } else if id_str.contains("GBP") {
            Currency::GBP
        } else if id_str.contains("JPY") {
            Currency::JPY
        } else {
            Currency::USD
        }
    };

    // Choose synthetic index
    let index_id = match currency {
        Currency::USD => "USD-SOFR",
        Currency::EUR => "EUR-ESTR",
        Currency::GBP => "GBP-SONIA",
        Currency::JPY => "JPY-TONA",
        _ => "USD-SOFR",
    }
    .to_string();

    // Synthesize quotes for each knot (excluding t≈0) and re-calibrate.
    // Use Deposit quotes for short maturities (<= 2Y) and Swap quotes for longer
    // maturities, matching the natural instrument coverage of a yield curve.

    let mut quotes = Vec::new();
    let dc = DayCount::Act365F;
    let dc_ctx = DayCountCtx::default();

    const SWAP_THRESHOLD_YEARS: f64 = 2.0;

    for &t in knots {
        if t <= 0.0001 {
            continue;
        }

        let df = curve.df(t);
        let maturity_days = (t * 365.25).round() as i64;
        let maturity = base_date + time::Duration::days(maturity_days);

        let yf = dc.year_fraction(base_date, maturity, dc_ctx).unwrap_or(t);

        if yf <= SWAP_THRESHOLD_YEARS {
            let rate = if yf > 1e-4 {
                (1.0 / df - 1.0) / yf
            } else {
                0.0
            };
            quotes.push(RateQuote::Deposit {
                id: QuoteId::new(format!("SYNTH-DEP-{}", t)),
                index: crate::market::conventions::ids::IndexId::new(index_id.clone()),
                pillar: Pillar::Date(maturity),
                rate,
            });
        } else {
            // Implied par swap rate: S = (DF_0 - DF_n) / Annuity
            // where Annuity = sum of DF(t_i) * tau_i over annual payment dates.
            let n_years = yf.round() as usize;
            let n_years = n_years.max(1);
            let mut annuity = 0.0;
            for i in 1..=n_years {
                let pay_t = i as f64;
                annuity += curve.df(pay_t);
            }
            let par_rate = if annuity > 1e-10 {
                (1.0 - df) / annuity
            } else {
                0.0
            };
            quotes.push(RateQuote::Swap {
                id: QuoteId::new(format!("SYNTH-SWP-{}", t)),
                index: crate::market::conventions::ids::IndexId::new(index_id.clone()),
                pillar: Pillar::Date(maturity),
                rate: par_rate,
                spread_decimal: None,
            });
        }
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
        },
    };

    bump_discount_curve(&quotes, &params, context, bump)
}

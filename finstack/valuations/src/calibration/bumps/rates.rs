//! Shared rates curve bumping logic.

use super::BumpRequest;
use crate::calibration::methods::DiscountCurveCalibrator;
use crate::calibration::quotes::InstrumentConventions;
use crate::calibration::{Calibrator, RatesQuote};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::types::Currency;

/// Bump discount curve by shocking par rates and re-calibrating.
pub fn bump_discount_curve(
    quotes: &[RatesQuote],
    calibrator: &DiscountCurveCalibrator,
    base_context: &MarketContext,
    bump: &BumpRequest,
    as_of: Date,
) -> finstack_core::Result<DiscountCurve> {
    // Clone quotes to apply bumps
    let mut bumped_quotes = quotes.to_vec();

    match bump {
        BumpRequest::Parallel(bp) => {
            let bump_decimal = bp * 1e-4;
            for q in &mut bumped_quotes {
                bump_quote_rate(q, bump_decimal);
            }
        }
        BumpRequest::Tenors(targets) => {
            // Sequential bumping for each target
            for (target_t, bp) in targets {
                if let Some(idx) = find_closest_quote(&bumped_quotes, *target_t, as_of) {
                    let bump_decimal = bp * 1e-4;
                    bump_quote_rate(&mut bumped_quotes[idx], bump_decimal);
                }
            }
        }
    }

    // Re-calibrate curve
    let (new_curve, _report) = calibrator.calibrate(&bumped_quotes, base_context)?;

    // We should ideally ensure the ID matches the original if passed, but
    // the calibrator returns a curve with the ID from the calibrator config.
    // The caller (scenarios/metrics) handles re-inserting with correct ID if needed.

    Ok(new_curve)
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

    // Synthesize quotes
    // We assume the curve is an OIS-like curve (risk-free).
    // We synthesize OIS Swaps for each knot point (except t=0).
    // Rate is implied par rate.
    // Ideally we use a helper to compute par rate from the curve itself.
    // ParRate = (FirstDF - LastDF) / PV01? For OIS?
    // For OIS, approx ParRate = -ln(DF)/T (continuously compounded) or simple compounding depending on daycount.
    // Let's use the `ZeroRate` as the proxy for the quote if we treat them as Zero Coupon generators?
    // Calibrator can take Deposits/Swaps.
    // If we synthesize Deposits for short end and OIS Swaps for long end.
    //
    // Simplification: Treat all points as OIS Swaps (1 payment at end? No, annual/freq).
    // OR: Synthesize "Zero Coupon OIS" (effectively calibration to ZC rates).
    // The `DiscountCurveCalibrator` handles standard instruments.
    // If we feed it `Swap` with `fixed_freq = Annual`, it expects an annual stream.
    //
    // Alternative: Use `Deposit` for everything? Deposit is simple interest ZC.
    // Rate = (1/DF - 1) * (360/Days).
    // This is robust for all points.
    // Let's use `RatesQuote::Deposit` for all knots. It simply converts DF to a simple rate.
    // This allows "perfect" round trip if we use Act365F or matching DC.

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

        // Apply bump
        let mut bumped_rate = rate;
        match bump {
            BumpRequest::Parallel(bp) => {
                bumped_rate += bp * 1e-4;
            }
            BumpRequest::Tenors(targets) => {
                for (target_t, bp) in targets {
                    if (t - *target_t).abs() < 0.1 {
                        bumped_rate += bp * 1e-4;
                    }
                }
            }
        }

        quotes.push(RatesQuote::Deposit {
            maturity,
            rate: bumped_rate,
            conventions: InstrumentConventions::default().with_day_count(dc),
        });
    }

    // Calibrate - disable spot knot to preserve original curve structure
    // (the input curve doesn't have a spot knot, so neither should the output)
    // Use settlement_days=0 because we're re-calibrating from an existing curve's
    // intrinsic discount factors, not from market quotes with settlement conventions.
    let calibrator = DiscountCurveCalibrator::new(curve_id.clone(), base_date, currency)
        .with_include_spot_knot(false)
        /* .with_settlement_days(0) */
        /* .with_allow_calendar_fallback(true) */;

    let (new_curve, _report) = calibrator.calibrate(&quotes, context)?;
    Ok(new_curve)
}

/// Bump the rate of a quote by the given amount (decimal).
pub fn bump_quote_rate(quote: &mut RatesQuote, bump_decimal: f64) {
    match quote {
        RatesQuote::Deposit { rate, .. } => *rate += bump_decimal,
        RatesQuote::FRA { rate, .. } => *rate += bump_decimal,
        RatesQuote::Future { price, .. } => *price -= bump_decimal * 100.0, // Price = 100 - rate%
        RatesQuote::Swap { rate, .. } => *rate += bump_decimal,
        RatesQuote::BasisSwap { spread_bp, .. } => *spread_bp += bump_decimal * 10_000.0,
    }
}

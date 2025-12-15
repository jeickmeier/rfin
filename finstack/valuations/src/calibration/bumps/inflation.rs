//! Shared inflation curve bumping logic.

use super::BumpRequest;
use crate::calibration::methods::InflationCurveCalibrator;
use crate::calibration::{Calibrator, InflationQuote};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::types::Currency;

use finstack_core::dates::Date;

/// Bump inflation curve by shocking implied zero-coupon swap rates and re-calibrating.
pub fn bump_inflation_rates(
    curve: &InflationCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    discount_id: &finstack_core::types::CurveId,
    as_of: Date,
) -> finstack_core::Result<InflationCurve> {
    let base_date = as_of;
    // Inflation curves usually don't carry their currency, but the calibrator needs it?
    // Actually InflationCurve doesn't have currency field.
    // InflationCurveCalibrator::new takes currency.
    // We might need to imply it or pass it.
    // For now, let's assume USD as default or try to infer from ID?
    // In scenarios, we often know the currency.
    // The curve ID usually contains the currency (e.g. "USD-CPI").
    // Let's rely on the caller to provide valid context, but here we just need to build quotes.
    // The calibrator needs to know the currency to look up the Discount Curve if it's currency dependent?
    // Actually InflationCurveCalibrator takes `currency` in `new`.

    // We'll try to parse currency from ID, or default to USD.
    let curve_id = curve.id();
    let id_str = curve_id.as_str();
    let currency = if id_str.contains("USD") {
        Currency::USD
    } else if id_str.contains("EUR") {
        Currency::EUR
    } else if id_str.contains("GBP") {
        Currency::GBP
    } else {
        Currency::USD // Fallback
    };

    let base_cpi = curve.base_cpi();
    let knots = curve.knots(); // time in years

    let mut quotes = Vec::new();

    for &t in knots {
        if t <= 0.0 {
            continue;
        } // Skip base point

        let cpi = curve.cpi(t);
        // Implied zero-coupon rate: (CPI(T) / Base)^(1/T) - 1
        let implied_rate = (cpi / base_cpi).powf(1.0 / t) - 1.0;

        let mut bumped_rate = implied_rate;

        // Apply bump
        match bump {
            BumpRequest::Parallel(bp) => {
                bumped_rate += bp * 1e-4;
            }
            BumpRequest::Tenors(targets) => {
                for (target_t, bp) in targets {
                    // 0.1 year tolerance
                    if (t - *target_t).abs() < 0.1 {
                        bumped_rate += bp * 1e-4;
                    }
                }
            }
        }

        let maturity_days = (t * 365.25).round() as i64;
        let maturity = base_date + time::Duration::days(maturity_days);

        quotes.push(InflationQuote::InflationSwap {
            maturity,
            rate: bumped_rate,
            index: curve_id.as_str().to_string(), // Use curve ID as index name? Or generic?
            conventions: Default::default(),
        });
    }

    if quotes.is_empty() {
        // No knots to bump? return clone
        return Ok(curve.clone());
    }

    // Calibrate new curve
    let calibrator = InflationCurveCalibrator::new(
        curve_id.clone(),
        base_date,
        currency,
        base_cpi,
        discount_id.clone(),
    );

    let (new_curve, _report) = calibrator.calibrate(&quotes, context)?;

    Ok(new_curve)
}

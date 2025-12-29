//! Shared inflation curve bumping logic.

use super::BumpRequest;
use crate::calibration::api::schema::{InflationCurveParams, StepParams};
use crate::calibration::config::CalibrationMethod;
use crate::calibration::targets::handlers::execute_step;
use crate::calibration::CalibrationConfig;
use crate::market::conventions::ids::InflationSwapConventionId;
use crate::market::quotes::inflation::InflationQuote;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::types::Currency;

use finstack_core::dates::Date;

/// Bump inflation curve by shocking implied zero-coupon swap rates and re-calibrating.
///
/// Converts the current inflation curve back to implied ZCIS rates,
/// applies shocks to those rates, and re-executes the [`InflationBootstrapper`].
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

    // Map currency to standard inflation convention ID
    let convention_id = match currency {
        Currency::USD => InflationSwapConventionId("USD-CPI".into()),
        Currency::EUR => InflationSwapConventionId("EUR-HICP".into()),
        Currency::GBP => InflationSwapConventionId("UK-RPI".into()),
        _ => InflationSwapConventionId(format!("{}-CPI", currency)), // Best guess fallback
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
            convention: convention_id.clone(),
        });
    }

    if quotes.is_empty() {
        // No knots to bump? return clone
        return Ok(curve.clone());
    }

    let market_quotes: Vec<MarketQuote> = quotes.into_iter().map(MarketQuote::Inflation).collect();
    let params = InflationCurveParams {
        curve_id: curve_id.clone(),
        currency,
        base_date,
        discount_curve_id: discount_id.clone(),
        index: curve_id.as_str().to_string(),
        observation_lag: "NONE".to_string(),
        base_cpi,
        notional: 1.0,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
    };

    let cfg = CalibrationConfig::default();
    let step = StepParams::Inflation(params.clone());
    let (ctx, _report) = execute_step(&step, &market_quotes, context, &cfg)?;
    Ok(ctx.get_inflation_ref(params.curve_id.as_str())?.clone())
}

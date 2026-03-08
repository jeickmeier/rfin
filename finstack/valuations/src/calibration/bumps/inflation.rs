//! Shared inflation curve bumping logic.

use super::BumpRequest;
use crate::calibration::api::schema::{InflationCurveParams, StepParams};
use crate::calibration::config::CalibrationMethod;
use crate::calibration::step_runtime;
use crate::calibration::CalibrationConfig;
use crate::market::conventions::ids::InflationSwapConventionId;
use crate::market::quotes::inflation::InflationQuote;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::InflationCurve;

use finstack_core::dates::Date;

/// Infer currency from an inflation curve ID using string heuristics.
///
/// This is a best-effort fallback for callers that don't have explicit currency
/// metadata. Returns USD if the curve ID doesn't match a known pattern.
pub fn infer_currency_from_curve_id(curve: &InflationCurve) -> Currency {
    let id_str = curve.id().as_str();
    if id_str.contains("USD") {
        Currency::USD
    } else if id_str.contains("EUR") {
        Currency::EUR
    } else if id_str.contains("GBP") {
        Currency::GBP
    } else {
        Currency::USD
    }
}

/// Derive the observation lag string from the curve's `indexation_lag_months`.
///
/// Returns `"NONE"` when the lag is 0, otherwise formats as `"{n}M"`.
pub fn observation_lag_from_curve(curve: &InflationCurve) -> String {
    let months = curve.indexation_lag_months();
    if months == 0 {
        "NONE".to_string()
    } else {
        format!("{months}M")
    }
}

/// Bump inflation curve by shocking implied zero-coupon swap rates and re-calibrating.
///
/// Converts the current inflation curve back to implied ZCIS rates,
/// applies shocks to those rates, and re-executes the inflation bootstrapper.
///
/// # Arguments
/// * `currency` - Currency of the inflation index (required; InflationCurve does not carry currency metadata).
/// * `observation_lag` - Observation lag string (e.g. "3M", "NONE") matching the original calibration.
pub fn bump_inflation_rates(
    curve: &InflationCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    discount_id: &finstack_core::types::CurveId,
    as_of: Date,
    currency: Currency,
    observation_lag: &str,
) -> finstack_core::Result<InflationCurve> {
    let base_date = as_of;
    let curve_id = curve.id();

    // Map currency to standard inflation convention ID
    let convention_id = match currency {
        Currency::USD => InflationSwapConventionId::new("USD-CPI"),
        Currency::EUR => InflationSwapConventionId::new("EUR-HICP"),
        Currency::GBP => InflationSwapConventionId::new("UK-RPI"),
        _ => InflationSwapConventionId::new(format!("{}-CPI", currency)), // Best guess fallback
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
        observation_lag: observation_lag.to_string(),
        base_cpi,
        notional: 1.0,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        seasonal_factors: None,
    };

    let cfg = CalibrationConfig::default();
    let step = StepParams::Inflation(params.clone());
    let (ctx, _report) =
        step_runtime::execute_params_and_apply(&step, &market_quotes, context, &cfg)?;
    Ok(ctx
        .get_inflation_curve(params.curve_id.as_str())?
        .as_ref()
        .clone())
}

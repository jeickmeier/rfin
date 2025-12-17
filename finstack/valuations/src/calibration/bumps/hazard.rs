//! Shared hazard curve bumping logic.

use super::BumpRequest;
use crate::calibration::adapters::handlers::execute_step;
use crate::calibration::config::CalibrationMethod;
use crate::calibration::api::schema::{HazardCurveParams, StepParams};
use crate::calibration::quotes::{CreditQuote, MarketQuote};
use crate::calibration::CalibrationConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::ParInterp;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::types::Currency;
use finstack_core::types::CurveId;

/// Bump hazard curve by shocking par spreads and re-calibrating.
///
/// Falls back to hazard rate shifting if par spread information is missing.
pub fn bump_hazard_spreads(
    hazard: &HazardCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    discount_id: Option<&CurveId>,
) -> finstack_core::Result<HazardCurve> {
    // Check if we have necessary data for re-calibration
    let par_points: Vec<(f64, f64)> = hazard.par_spread_points().collect();

    let Some(discount_id) = discount_id else {
        // Fallback to hazard rate shift (Model Sensitivity)
        return bump_hazard_shift_fallback(hazard, bump);
    };

    if par_points.is_empty() {
        // Fallback if no par points
        return bump_hazard_shift_fallback(hazard, bump);
    }

    // Construct CreditQuotes from par points with bumps applied
    let base_date = hazard.base_date();
    let currency = hazard.currency().unwrap_or(Currency::USD);
    let recovery = hazard.recovery_rate();
    let seniority = hazard.seniority.unwrap_or(Seniority::Senior);
    let issuer = hazard
        .issuer()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let mut quotes = Vec::new();

    for (tenor_years, spread_bp) in par_points {
        let maturity_days = (tenor_years * 365.25).round() as i64;
        let maturity = base_date + time::Duration::days(maturity_days);

        let mut bumped_spread = spread_bp;

        // Apply bump
        match bump {
            BumpRequest::Parallel(bp) => {
                bumped_spread += bp;
            }
            BumpRequest::Tenors(targets) => {
                for (target_t, bp) in targets {
                    // 0.1 year tolerance for bucket matching
                    if (tenor_years - target_t).abs() < 0.1 {
                        bumped_spread += bp;
                        // Assuming we want to apply multiple bumps if they overlap,
                        // or just the first match?
                        // Usually buckets are distinct. Let's allow sum if multiple match?
                        // No, best to break or just sum. Sum is safer for complex requests.
                    }
                }
            }
        }

        quotes.push(CreditQuote::CDS {
            entity: issuer.clone(),
            currency,
            maturity,
            spread_bp: bumped_spread,
            recovery_rate: recovery,
            conventions: Default::default(),
        });
    }

    let market_quotes: Vec<MarketQuote> = quotes.into_iter().map(MarketQuote::Credit).collect();
    let params = HazardCurveParams {
        curve_id: hazard.id().clone(),
        entity: issuer,
        seniority,
        currency,
        base_date,
        discount_curve_id: discount_id.clone(),
        recovery_rate: recovery,
        notional: 1.0,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        par_interp: ParInterp::Linear,
    };

    let cfg = CalibrationConfig::default();
    let step = StepParams::Hazard(params.clone());
    let (ctx, _report) = execute_step(&step, &market_quotes, context, &cfg)?;
    Ok(ctx.get_hazard_ref(params.curve_id.as_str())?.clone())
}

/// Fallback: bump hazard rates directly (Model Sensitivity).
fn bump_hazard_shift_fallback(
    hazard: &HazardCurve,
    bump: &BumpRequest,
) -> finstack_core::Result<HazardCurve> {
    match bump {
        BumpRequest::Parallel(bp) => {
            // Convert bp to decimal
            let bump_decimal = bp * 1e-4;
            let temp_bumped = hazard.with_hazard_shift(bump_decimal)?;
            temp_bumped
                .to_builder_with_id(hazard.id().clone())
                .build()
                .map_err(|_| finstack_core::Error::Internal)
        }
        BumpRequest::Tenors(targets) => {
            // Sequential bumping for each target
            let mut current = hazard.clone();
            for (t, bp) in targets {
                current = with_key_rate_hazard_bump(&current, *t, *bp)?;
            }
            Ok(current)
        }
    }
}

/// Helper to apply a key-rate bump to a hazard curve at a specific tenor.
fn with_key_rate_hazard_bump(
    hazard: &HazardCurve,
    t_years: f64,
    bump_bp: f64,
) -> finstack_core::Result<HazardCurve> {
    // Convert bump from bp to hazard rate units
    let bump_decimal = bump_bp * 1e-4;

    let knots: Vec<f64> = hazard.knot_points().map(|(t, _)| t).collect();
    let hazard_rates: Vec<f64> = hazard.knot_points().map(|(_, lambda)| lambda).collect();

    if knots.len() < 2 {
        return hazard.with_hazard_shift(bump_decimal);
    }

    let mut target_segment = 0usize;
    if t_years <= knots[0] {
        target_segment = 0;
    } else if t_years >= knots[knots.len() - 1] {
        target_segment = knots.len() - 2;
    } else {
        for i in 0..knots.len() - 1 {
            if t_years > knots[i] && t_years <= knots[i + 1] {
                target_segment = i;
                break;
            }
        }
    }

    let mut bumped_rates = hazard_rates;
    bumped_rates[target_segment] = (bumped_rates[target_segment] + bump_decimal).max(0.0);

    let bumped_points: Vec<(f64, f64)> = knots
        .iter()
        .zip(bumped_rates.iter())
        .map(|(&t, &lambda)| (t, lambda))
        .collect();

    let mut builder = hazard
        .to_builder_with_id(hazard.id().clone())
        .knots(bumped_points);

    builder = builder.par_spreads(hazard.par_spread_points());

    builder
        .build()
        .map_err(|_e| finstack_core::Error::from(finstack_core::error::InputError::Invalid))
}

//! Shared hazard curve bumping logic.

use super::BumpRequest;
use crate::calibration::api::schema::{HazardCurveParams, StepParams};
use crate::calibration::config::CalibrationMethod;
use crate::calibration::step_runtime;
use crate::calibration::CalibrationConfig;
use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::currency::Currency;
use finstack_core::dates::{Tenor, TenorUnit};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::term_structures::ParInterp;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::types::CurveId;

/// Bump hazard curve by shocking par spreads and re-calibrating.
///
/// This is the standard "Risk Re-calibration" approach. It extracts par
/// points from the current curve, applies shocks, and builds a new
/// [`CreditQuote`] set to solve for a new hazard curve.
///
/// This function is strictly recalibration-only; callers that want a
/// model hazard shift should call [`bump_hazard_shift`] explicitly.
pub fn bump_hazard_spreads(
    hazard: &HazardCurve,
    context: &MarketContext,
    bump: &BumpRequest,
    discount_id: Option<&CurveId>,
) -> finstack_core::Result<HazardCurve> {
    // Check if we have necessary data for re-calibration
    let par_points: Vec<(f64, f64)> = hazard.par_spread_points().collect();

    let Some(discount_id) = discount_id else {
        return Err(finstack_core::Error::Input(
            finstack_core::InputError::NotFound {
                id: "discount curve for hazard recalibration".to_string(),
            },
        ));
    };

    if par_points.is_empty() {
        return Err(finstack_core::Error::Input(
            finstack_core::InputError::TooFewPoints,
        ));
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
        // Snap irregular year-fractions (often coming from date-based calibration) to standard
        // CDS tenors for stable schedule generation and deterministic bump matching.
        let raw_months = (tenor_years * 12.0).round().max(1.0) as i32;
        const STD_MONTHS: [i32; 11] = [3, 6, 12, 24, 36, 60, 84, 120, 180, 240, 360];
        let mut snapped_months = raw_months;
        if let Some(best) = STD_MONTHS
            .iter()
            .copied()
            .min_by(|a, b| (raw_months - a).abs().cmp(&(raw_months - b).abs()))
        {
            if (raw_months - best).abs() <= 2 {
                snapped_months = best;
            } else {
                // Fallback: nearest quarter-year multiple.
                snapped_months = ((raw_months as f64 / 3.0).round() as i32).max(1) * 3;
            }
        }
        let snapped_years = snapped_months as f64 / 12.0;

        let mut bumped_spread = spread_bp;

        // Apply bump
        match bump {
            BumpRequest::Parallel(bp) => {
                bumped_spread += bp;
            }
            BumpRequest::Tenors(targets) => {
                for (target_t, bp) in targets {
                    // 0.1 year tolerance for bucket matching
                    if (snapped_years - target_t).abs() < 0.1 {
                        bumped_spread += bp;
                        // Assuming we want to apply multiple bumps if they overlap,
                        // or just the first match?
                        // Usually buckets are distinct. Let's allow sum if multiple match?
                        // No, best to break or just sum. Sum is safer for complex requests.
                    }
                }
            }
        }

        quotes.push(CdsQuote::CdsParSpread {
            id: format!("BUMP-{}-{:.4}", issuer, snapped_years).into(),
            entity: issuer.clone(),
            // Use tenor pillars so CDS schedule generation can snap to market-standard
            // IMM maturities. Using ad-hoc `Date` pillars can create invalid
            // ranges (e.g. maturity before the next IMM coupon) and make the
            // hazard bootstrap fail.
            pillar: crate::market::quotes::ids::Pillar::Tenor(Tenor::new(
                snapped_months as u32,
                TenorUnit::Months,
            )),
            spread_bp: bumped_spread,
            recovery_rate: recovery,
            convention: crate::market::conventions::ids::CdsConventionKey {
                currency,
                doc_clause: crate::market::conventions::ids::CdsDocClause::IsdaNa,
            },
        });
    }

    let market_quotes: Vec<MarketQuote> = quotes.into_iter().map(MarketQuote::Cds).collect();
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
        doc_clause: None,
    };

    let cfg = CalibrationConfig::default();
    let step = StepParams::Hazard(params.clone());
    let (ctx, _report) =
        step_runtime::execute_params_and_apply(&step, &market_quotes, context, &cfg)?;
    let new_curve = ctx.get_hazard(params.curve_id.as_str())?.as_ref().clone();
    Ok(new_curve)
}

/// Bump hazard curve directly (model hazard shift), without recalibration.
pub fn bump_hazard_shift(
    hazard: &HazardCurve,
    bump: &BumpRequest,
) -> finstack_core::Result<HazardCurve> {
    bump_hazard_shift_fallback(hazard, bump)
}

/// Fallback: bump hazard rates directly (Model Sensitivity / Hazard Delta).
fn bump_hazard_shift_fallback(
    hazard: &HazardCurve,
    bump: &BumpRequest,
) -> finstack_core::Result<HazardCurve> {
    match bump {
        BumpRequest::Parallel(bp) => {
            // Convert bp to decimal
            let bump_decimal = bp * 1e-4;
            let temp_bumped = hazard.with_parallel_bump(bump_decimal)?;
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
        return hazard.with_parallel_bump(bump_decimal);
    }

    // If the requested bucket is beyond the curve's supported maturity, treat as "no-op".
    // This avoids double-counting in bucketed CS01 when requesting standard buckets
    // beyond the last calibrated hazard knot.
    #[allow(clippy::expect_used)] // Infallible after len>=2 check above
    let last_knot = *knots.last().expect("len>=2");
    if t_years > last_knot + 1e-6 {
        return Ok(hazard.clone());
    }

    // If the request matches an existing knot, bump that knot directly.
    // Otherwise bump the segment that contains the target time.
    let eps = 1e-6;
    let mut target_idx = knots
        .iter()
        .position(|&k| (k - t_years).abs() <= eps)
        .unwrap_or(0);
    if target_idx == 0 {
        if t_years <= knots[0] {
            target_idx = 0;
        } else if t_years >= knots[knots.len() - 1] {
            target_idx = knots.len() - 1;
        } else {
            for i in 0..knots.len() - 1 {
                if t_years > knots[i] && t_years < knots[i + 1] {
                    target_idx = i;
                    break;
                }
            }
        }
    }

    let mut bumped_rates = hazard_rates;
    bumped_rates[target_idx] = (bumped_rates[target_idx] + bump_decimal).max(0.0);

    let bumped_points: Vec<(f64, f64)> = knots
        .iter()
        .zip(bumped_rates.iter())
        .map(|(&t, &lambda)| (t, lambda))
        .collect();

    let mut builder = HazardCurve::builder(hazard.id().clone())
        .base_date(hazard.base_date())
        .recovery_rate(hazard.recovery_rate())
        .day_count(hazard.day_count())
        .knots(bumped_points)
        .par_interp(hazard.par_interp())
        .par_spreads(hazard.par_spread_points());

    if let Some(issuer) = hazard.issuer() {
        builder = builder.issuer(issuer.to_string());
    }
    if let Some(seniority) = hazard.seniority {
        builder = builder.seniority(seniority);
    }
    if let Some(currency) = hazard.currency() {
        builder = builder.currency(currency);
    }

    builder
        .build()
        .map_err(|_e| finstack_core::Error::from(finstack_core::InputError::Invalid))
}

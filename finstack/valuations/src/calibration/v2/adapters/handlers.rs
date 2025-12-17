//! Calibration handlers (adapters).
//!
//! Maps API steps to domain logic execution.

use crate::calibration::config::CalibrationConfig;
use crate::calibration::methods::discount::default_curve_day_count;
use crate::calibration::v2::adapters::base_correlation::BaseCorrelationBootstrapper;
use crate::calibration::v2::adapters::discount::{DiscountCurveTarget, DiscountCurveTargetParams};
use crate::calibration::v2::adapters::forward::{ForwardCurveTarget, ForwardCurveTargetParams};
use crate::calibration::v2::adapters::hazard::HazardBootstrapper;
use crate::calibration::v2::adapters::inflation::InflationBootstrapper;
use crate::calibration::v2::adapters::swaption::SwaptionVolAdapter;
use crate::calibration::v2::adapters::vol::VolSurfaceAdapter;
use crate::calibration::v2::api::schema::{CalibrationMethod, RatesStepConventions, StepParams};
use crate::calibration::v2::domain::pricing::CalibrationPricer;
use crate::calibration::v2::domain::quotes::{ExtractQuotes, MarketQuote};
use crate::calibration::v2::domain::solver::{
    BootstrapTarget, GlobalFitOptimizer, SequentialBootstrapper,
};
use crate::calibration::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::prelude::*;

fn err_too_few_points() -> finstack_core::Error {
    finstack_core::Error::Input(finstack_core::error::InputError::TooFewPoints)
}

fn require_non_empty<T>(quotes: &[T]) -> Result<()> {
    if quotes.is_empty() {
        Err(err_too_few_points())
    } else {
        Ok(())
    }
}

/// Sort bootstrap quotes by strictly increasing knot time.
///
/// Market-standard behavior: quote ordering should not affect calibration outcomes.
/// The core bootstrapper assumes quotes are already sorted, so we enforce that here
/// centrally for all bootstrap-based steps.
fn sort_bootstrap_quotes<T: BootstrapTarget>(target: &T, quotes: &mut Vec<T::Quote>) -> Result<()> {
    if quotes.len() <= 1 {
        return Ok(());
    }

    // Drain + compute times once, then stable-sort by time with deterministic tie-breaker.
    let mut items: Vec<(f64, usize, T::Quote)> = Vec::with_capacity(quotes.len());
    for (idx, q) in quotes.drain(..).enumerate() {
        let t = target.quote_time(&q)?;
        if !t.is_finite() || t <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!("Bootstrap quote_time must be finite and > 0; got t={}", t),
                category: "bootstrapping".to_string(),
            });
        }
        items.push((t, idx, q));
    }

    items.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    // Enforce strict monotonicity (matches `SequentialBootstrapper` requirements).
    let mut last_t = 0.0_f64;
    for (t, _, _) in &items {
        if *t <= last_t {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Bootstrap requires strictly increasing quote times; got t={:.12} after last_time={:.12}",
                    t, last_t
                ),
                category: "bootstrapping".to_string(),
            });
        }
        last_t = *t;
    }

    quotes.extend(items.into_iter().map(|(_, _, q)| q));
    Ok(())
}

fn run_bootstrap<T: BootstrapTarget>(
    target: &T,
    quotes: &[T::Quote],
    initial_knots: Vec<(f64, f64)>,
    config: &CalibrationConfig,
) -> Result<(T::Curve, CalibrationReport)> {
    let trace = if config.explain.enabled {
        Some(finstack_core::explain::ExplanationTrace::new(
            "calibration_v2",
        ))
    } else {
        None
    };
    SequentialBootstrapper::bootstrap(target, quotes, initial_knots, config, trace)
}

pub(crate) fn apply_rates_step_conventions(
    mut pricer: CalibrationPricer,
    currency: finstack_core::types::Currency,
    conventions: &RatesStepConventions,
    default_use_settlement_start: bool,
) -> Result<CalibrationPricer> {
    if let Some(days) = conventions.settlement_days {
        pricer = pricer.with_settlement_days(days);
    }
    if let Some(calendar_id) = &conventions.calendar_id {
        pricer = pricer.with_calendar_id(calendar_id.clone());
    }
    if let Some(bdc) = conventions.business_day_convention {
        pricer = pricer.with_business_day_convention(bdc);
    }
    if let Some(allow) = conventions.allow_calendar_fallback {
        pricer = pricer.with_allow_calendar_fallback(allow);
    }
    pricer = pricer.with_use_settlement_start(
        conventions
            .use_settlement_start
            .unwrap_or(default_use_settlement_start),
    );
    if let Some(days) = conventions.default_payment_delay_days {
        pricer = pricer.with_default_payment_delay_days(days);
    }
    if let Some(days) = conventions.default_reset_lag_days {
        pricer = pricer.with_default_reset_lag_days(days);
    }
    if let Some(params) = &conventions.convexity_params {
        pricer = pricer.with_convexity_params(params.clone());
    }

    let strict_pricing = conventions.strict_pricing.unwrap_or(false);
    pricer = pricer.with_strict_pricing(strict_pricing);

    if strict_pricing {
        if pricer.settlement_days.is_none() {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires step-level settlement_days to be set".to_string(),
            ));
        }
        if pricer.calendar_id.is_none() {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires step-level calendar_id to be set".to_string(),
            ));
        }
        if pricer.business_day_convention.is_none() {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires step-level business_day_convention to be set".to_string(),
            ));
        }
        if pricer.default_payment_delay_days.is_none() {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires step-level default_payment_delay_days to be set"
                    .to_string(),
            ));
        }
        if pricer.default_reset_lag_days.is_none() {
            return Err(finstack_core::Error::Validation(
                "Strict pricing requires step-level default_reset_lag_days to be set".to_string(),
            ));
        }
        Ok(pricer)
    } else {
        // Populate remaining pricer defaults (currency-specific)
        Ok(pricer.with_market_conventions(currency))
    }
}

pub(crate) fn discount_curve_day_count(
    currency: finstack_core::types::Currency,
    conventions: &RatesStepConventions,
) -> finstack_core::dates::DayCount {
    conventions
        .curve_day_count
        .unwrap_or_else(|| default_curve_day_count(currency))
}

/// Execute a single calibration step.
pub fn execute_step(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    match params {
        StepParams::Discount(p) => {
            let mut rates_quotes = quotes.extract_quotes();
            require_non_empty(&rates_quotes)?;

            let pricer = CalibrationPricer::new(p.base_date, p.curve_id.clone())
                .with_discount_curve_id(p.pricing_discount_id.clone().unwrap_or(p.curve_id.clone()))
                .with_forward_curve_id(p.pricing_forward_id.clone().unwrap_or(p.curve_id.clone()));
            let pricer = apply_rates_step_conventions(pricer, p.currency, &p.conventions, true)?;

            let curve_dc = discount_curve_day_count(p.currency, &p.conventions);
            let settlement = pricer.settlement_date(p.currency)?;

            let target = DiscountCurveTarget::new(DiscountCurveTargetParams {
                base_date: p.base_date,
                currency: p.currency,
                curve_id: p.curve_id.clone(),
                discount_curve_id: p.pricing_discount_id.clone().unwrap_or(p.curve_id.clone()),
                forward_curve_id: p.pricing_forward_id.clone().unwrap_or(p.curve_id.clone()),
                solve_interp: p.interpolation,
                extrapolation: p.extrapolation,
                config: global_config.clone(),
                pricer,
                curve_day_count: curve_dc,
                spot_knot: None,
                settlement_date: settlement,
                base_context: context.clone(),
            });

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => {
                    sort_bootstrap_quotes(&target, &mut rates_quotes)?;
                    run_bootstrap(&target, &rates_quotes, vec![(0.0, 1.0)], global_config)?
                }
                CalibrationMethod::Global => {
                    GlobalFitOptimizer::optimize(&target, &rates_quotes, global_config)?
                }
            };

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
        StepParams::Forward(p) => {
            let mut rates_quotes = quotes.extract_quotes();
            require_non_empty(&rates_quotes)?;

            let pricer = CalibrationPricer::for_forward_curve(
                p.base_date,
                p.curve_id.clone(),
                p.discount_curve_id.clone(),
                p.tenor_years,
            );
            let pricer = apply_rates_step_conventions(pricer, p.currency, &p.conventions, false)?;

            let curve_dc = discount_curve_day_count(p.currency, &p.conventions);

            let target = ForwardCurveTarget::new(ForwardCurveTargetParams {
                base_date: p.base_date,
                currency: p.currency,
                fwd_curve_id: p.curve_id.clone(),
                discount_curve_id: p.discount_curve_id.clone(),
                tenor_years: p.tenor_years,
                solve_interp: p.interpolation,
                config: global_config.clone(),
                pricer,
                time_day_count: curve_dc,
                base_context: context.clone(),
            });

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => {
                    sort_bootstrap_quotes(&target, &mut rates_quotes)?;
                    run_bootstrap(&target, &rates_quotes, Vec::new(), global_config)?
                }
                CalibrationMethod::Global => {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ));
                }
            };

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
        StepParams::Hazard(p) => {
            let mut credit_quotes = quotes.extract_quotes();
            require_non_empty(&credit_quotes)?;

            let target = HazardBootstrapper::new(p.clone(), context.clone());

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => {
                    sort_bootstrap_quotes(&target, &mut credit_quotes)?;
                    run_bootstrap(&target, &credit_quotes, Vec::new(), global_config)?
                }
                CalibrationMethod::Global => {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ));
                }
            };

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
        StepParams::Inflation(p) => {
            let mut inflation_quotes = quotes.extract_quotes();
            require_non_empty(&inflation_quotes)?;

            let target = InflationBootstrapper::new(p.clone(), context.clone());

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => run_bootstrap(
                    &target,
                    {
                        sort_bootstrap_quotes(&target, &mut inflation_quotes)?;
                        &inflation_quotes
                    },
                    vec![(0.0, p.base_cpi)],
                    global_config,
                )?,
                CalibrationMethod::Global => {
                    return Err(finstack_core::Error::Input(
                        finstack_core::error::InputError::Invalid,
                    ));
                }
            };

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
        StepParams::VolSurface(p) => {
            let (surface, report) =
                VolSurfaceAdapter::calibrate(p, quotes, context, global_config)?;
            let mut new_context = context.clone();
            new_context.insert_surface_mut(std::sync::Arc::new(surface));
            Ok((new_context, report))
        }
        StepParams::SwaptionVol(p) => {
            let (surface, report) =
                SwaptionVolAdapter::calibrate(p, quotes, context, global_config)?;
            let mut new_context = context.clone();
            new_context.insert_surface_mut(std::sync::Arc::new(surface));
            Ok((new_context, report))
        }
        StepParams::BaseCorrelation(p) => {
            let mut credit_quotes = quotes.extract_quotes();
            require_non_empty(&credit_quotes)?;

            let target = BaseCorrelationBootstrapper::new(p.clone(), context.clone());

            sort_bootstrap_quotes(&target, &mut credit_quotes)?;
            let (curve, report) =
                run_bootstrap(&target, &credit_quotes, Vec::new(), global_config)?;

            let mut new_context = context.clone();
            let arc = std::sync::Arc::new(curve);
            new_context.insert_mut(std::sync::Arc::clone(&arc));

            // Ensure downstream pricing sees the calibrated curve via the credit index aggregate.
            if let Ok(index) = new_context.credit_index_ref(&p.index_id) {
                let updated = CreditIndexData {
                    base_correlation_curve: std::sync::Arc::clone(&arc),
                    ..index.clone()
                };
                new_context.insert_credit_index_mut(&p.index_id, updated);
            }
            Ok((new_context, report))
        }
    }
}

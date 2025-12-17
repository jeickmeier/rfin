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
use crate::calibration::v2::api::schema::{CalibrationMethod, StepParams};
use crate::calibration::v2::domain::pricing::CalibrationPricer;
use crate::calibration::v2::domain::quotes::{ExtractQuotes, MarketQuote};
use crate::calibration::v2::domain::solver::{GlobalOptimizer, SequentialBootstrapper};
use crate::calibration::CalibrationReport;
use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;

/// Execute a single calibration step.
pub fn execute_step(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    match params {
        StepParams::Discount(p) => {
            let rates_quotes = quotes.extract_quotes();
            if rates_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let pricer = CalibrationPricer::new(p.base_date, p.curve_id.clone())
                .with_market_conventions(p.currency)
                .with_discount_curve_id(p.pricing_discount_id.clone().unwrap_or(p.curve_id.clone()))
                .with_forward_curve_id(p.pricing_forward_id.clone().unwrap_or(p.curve_id.clone()));

            let curve_dc = default_curve_day_count(p.currency);
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
                    let initial_knots = vec![(0.0, 1.0)];
                    SequentialBootstrapper::bootstrap(
                        &target,
                        &rates_quotes,
                        initial_knots,
                        global_config,
                        None,
                    )?
                }
                CalibrationMethod::Global => {
                    GlobalOptimizer::optimize(&target, &rates_quotes, global_config)?
                }
            };

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
        StepParams::Forward(p) => {
            let rates_quotes = quotes.extract_quotes();
            if rates_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let pricer = CalibrationPricer::for_forward_curve(
                p.base_date,
                p.curve_id.clone(),
                p.discount_curve_id.clone(),
                p.tenor_years,
            )
            .with_market_conventions(p.currency);

            let curve_dc = default_curve_day_count(p.currency);

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
                CalibrationMethod::Bootstrap => SequentialBootstrapper::bootstrap(
                    &target,
                    &rates_quotes,
                    Vec::new(),
                    global_config,
                    None,
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
        StepParams::Hazard(p) => {
            let credit_quotes = quotes.extract_quotes();
            if credit_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let target = HazardBootstrapper::new(
                p.clone(),
                credit_quotes,
                global_config.clone(),
                context.clone(),
            );

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => {
                    SequentialBootstrapper::bootstrap(
                        &target,
                        target.instruments(), // HazardBootstrapper holds the quotes but SequentialBootstrapper needs reference
                        Vec::new(),
                        global_config,
                        None,
                    )?
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
            let inflation_quotes = quotes.extract_quotes();
            if inflation_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let target = InflationBootstrapper::new(
                p.clone(),
                inflation_quotes, // Consumes quotes
                global_config.clone(),
                context.clone(),
            );

            let (curve, report) = match p.method {
                CalibrationMethod::Bootstrap => SequentialBootstrapper::bootstrap(
                    &target,
                    target.instruments(),
                    vec![(0.0, p.base_cpi)],
                    global_config,
                    None,
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
            let credit_quotes = quotes.extract_quotes();
            if credit_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let target = BaseCorrelationBootstrapper::new(
                p.clone(),
                credit_quotes,
                global_config.clone(),
                context.clone(),
            );

            let (curve, report) = SequentialBootstrapper::bootstrap(
                &target,
                target.instruments(), // Needs to expose instruments
                Vec::new(),
                global_config,
                None,
            )?;

            let mut new_context = context.clone();
            new_context.insert_mut(curve);
            Ok((new_context, report))
        }
    }
}

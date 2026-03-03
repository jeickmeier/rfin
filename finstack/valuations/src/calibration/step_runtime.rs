use crate::calibration::api::schema::{CalibrationStep, StepParams};
use crate::calibration::config::CalibrationConfig;
use crate::calibration::targets::base_correlation::BaseCorrelationBootstrapper;
use crate::calibration::targets::discount::DiscountCurveTarget;
use crate::calibration::targets::forward::ForwardCurveTarget;
use crate::calibration::targets::hazard::HazardBootstrapper;
use crate::calibration::targets::inflation::InflationBootstrapper;
use crate::calibration::targets::student_t::StudentTCalibrator;
use crate::calibration::targets::swaption::SwaptionVolBootstrapper;
use crate::calibration::targets::vol::VolSurfaceBootstrapper;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::explain::TraceEntry;
use finstack_core::market_data::context::{CurveStorage, MarketContext};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::types::CurveId;
use finstack_core::Result;
use std::sync::Arc;

/// Normalized output key for a step.
pub(crate) enum OutputKey {
    Curve(CurveId),
    Surface(CurveId),
    Scalar(String),
}

/// Normalized output payload for a step.
pub(crate) enum StepOutput {
    Curve(CurveStorage),
    Surface(Arc<VolSurface>),
    Scalar { key: String, value: MarketScalar },
}

/// Aggregated outcome of a single calibration step.
pub(crate) struct StepOutcome {
    pub output: StepOutput,
    pub credit_index_update: Option<(String, CreditIndexData)>,
    pub report: CalibrationReport,
}

/// Compute the output key for batching without executing the step.
pub(crate) fn output_key(step: &CalibrationStep) -> OutputKey {
    match &step.params {
        StepParams::Discount(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Forward(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Hazard(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Inflation(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::BaseCorrelation(p) => {
            OutputKey::Curve(CurveId::from(format!("{}_CORR", p.index_id)))
        }
        StepParams::VolSurface(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
        StepParams::SwaptionVol(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
        StepParams::StudentT(p) => {
            OutputKey::Scalar(format!("{}_STUDENT_T_DF", p.tranche_instrument_id))
        }
    }
}

/// Apply a normalized step output into the mutable market context.
pub(crate) fn apply_output(
    context: &mut MarketContext,
    output: StepOutput,
    credit_index_update: Option<(String, CreditIndexData)>,
) {
    match output {
        StepOutput::Curve(curve) => {
            *context = std::mem::take(context).insert(curve);
        }
        StepOutput::Surface(surface) => {
            *context = std::mem::take(context).insert_surface(surface);
        }
        StepOutput::Scalar { key, value } => {
            *context = std::mem::take(context).insert_price(&key, value);
        }
    }

    if let Some((id, data)) = credit_index_update {
        *context = std::mem::take(context).insert_credit_index(id, data);
    }
}

/// Execute calibration logic for the provided [`StepParams`].
pub(crate) fn execute_params(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<StepOutcome> {
    match params {
        StepParams::Discount(p) => {
            let (ctx, report) = DiscountCurveTarget::solve(p, quotes, context, global_config)?;
            let output = StepOutput::Curve(ctx.get_discount(&p.curve_id)?.into());
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Forward(p) => {
            let (ctx, report) = ForwardCurveTarget::solve(p, quotes, context, global_config)?;
            let output = StepOutput::Curve(ctx.get_forward(&p.curve_id)?.into());
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Hazard(p) => {
            let (ctx, report) = HazardBootstrapper::solve(p, quotes, context, global_config)?;
            let output = StepOutput::Curve(ctx.get_hazard(&p.curve_id)?.into());
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::Inflation(p) => {
            let (ctx, report) = InflationBootstrapper::solve(p, quotes, context, global_config)?;
            let output = StepOutput::Curve(ctx.get_inflation(&p.curve_id)?.into());
            Ok(StepOutcome {
                output,
                credit_index_update: None,
                report,
            })
        }
        StepParams::BaseCorrelation(p) => {
            let (ctx, report) =
                BaseCorrelationBootstrapper::solve(p, quotes, context, global_config)?;
            let curve_id = CurveId::from(format!("{}_CORR", p.index_id));
            let output = StepOutput::Curve(ctx.get_base_correlation(curve_id.as_str())?.into());
            let credit_index_update = ctx
                .credit_index(&p.index_id)
                .ok()
                .map(|idx| (p.index_id.clone(), idx.as_ref().clone()));
            Ok(StepOutcome {
                output,
                credit_index_update,
                report,
            })
        }
        StepParams::VolSurface(p) => {
            let (surface, report) =
                VolSurfaceBootstrapper::solve(p, quotes, context, global_config)?;
            // Preserve context insertion behavior
            let mut new_report = report.clone();
            new_report
                .explanation
                .get_or_insert_with(|| finstack_core::explain::ExplanationTrace::new("vol_surface"))
                .push(
                    TraceEntry::ComputationStep {
                        name: "surface_built".to_string(),
                        description: "Vol surface constructed".to_string(),
                        metadata: None,
                    },
                    global_config.explain.max_entries,
                );
            Ok(StepOutcome {
                output: StepOutput::Surface(surface.into()),
                credit_index_update: None,
                report: new_report,
            })
        }
        StepParams::SwaptionVol(p) => {
            let (surface, report) =
                SwaptionVolBootstrapper::solve(p, quotes, context, global_config)?;
            Ok(StepOutcome {
                output: StepOutput::Surface(surface.into()),
                credit_index_update: None,
                report,
            })
        }
        StepParams::StudentT(p) => {
            let (_, calibrated_df, report) =
                StudentTCalibrator::solve(p, quotes, context, global_config)?;
            let scalar_key = format!("{}_STUDENT_T_DF", p.tranche_instrument_id);
            Ok(StepOutcome {
                output: StepOutput::Scalar {
                    key: scalar_key,
                    value: MarketScalar::Unitless(calibrated_df),
                },
                credit_index_update: None,
                report,
            })
        }
    }
}

/// Execute a calibration step and normalize its output/result.
pub(crate) fn execute(
    step: &CalibrationStep,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<StepOutcome> {
    execute_params(&step.params, quotes, context, global_config)
}

/// Execute [`StepParams`] directly and apply the output to a cloned context.
pub(crate) fn execute_params_and_apply(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    let outcome = execute_params(params, quotes, context, global_config)?;
    let StepOutcome {
        output,
        credit_index_update,
        report,
    } = outcome;

    let mut new_context = context.clone();
    apply_output(&mut new_context, output, credit_index_update);
    Ok((new_context, report))
}

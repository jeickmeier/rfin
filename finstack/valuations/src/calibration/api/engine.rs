//! Calibration execution engine.
//!
//! Orchestrates the execution of a calibration plan.

use super::schema::CalibrationEnvelope;
use crate::calibration::api::schema::CalibrationStep;
use crate::calibration::api::schema::{CalibrationResult, CalibrationResultEnvelope, StepParams};
use crate::calibration::targets::handlers::execute_step;
use crate::calibration::targets::util::curve_day_count_from_quotes;
// use crate::calibration::pricing::{CalibrationPricer, RatesQuoteUseCase}; // Removed
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::{CurveStorage, MarketContext};
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
use finstack_core::prelude::*;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// Merges explanation traces from individual calibration steps into a plan-level trace.
fn merge_step_traces(
    step_reports: &BTreeMap<String, CalibrationReport>,
    config: &crate::calibration::config::CalibrationConfig,
) -> Option<ExplanationTrace> {
    if !config.explain.enabled {
        return None;
    }

    let mut merged = ExplanationTrace::new("calibration_plan");
    for (step_id, report) in step_reports {
        merged.push(
            TraceEntry::ComputationStep {
                name: format!("step:{step_id}"),
                description: "Begin step trace".to_string(),
                metadata: None,
            },
            config.explain.max_entries,
        );

        if let Some(step_trace) = report.explanation.as_ref() {
            for entry in &step_trace.entries {
                merged.push(entry.clone(), config.explain.max_entries);
            }
            if step_trace.is_truncated() {
                merged.truncated = Some(true);
            }
        }
    }
    Some(merged)
}

/// Aggregates per-step reports into a single plan execution report.
fn aggregate_plan_report(
    aggregated_residuals: BTreeMap<String, f64>,
    total_iterations: usize,
    step_reports: &BTreeMap<String, CalibrationReport>,
    config: &crate::calibration::config::CalibrationConfig,
) -> CalibrationReport {
    let all_steps_success = step_reports.values().all(|r| r.success);
    let all_steps_validation_passed = step_reports.values().all(|r| r.validation_passed);

    // Market-standard: plan-level success follows per-step success. The plan's solver tolerance
    // is not meaningful for aggregating residuals across heterogeneous steps (rates, credit, vols).
    let mut report = CalibrationReport::new(
        aggregated_residuals,
        total_iterations,
        all_steps_success && all_steps_validation_passed,
        if all_steps_success && all_steps_validation_passed {
            "Plan execution completed"
        } else {
            "Plan execution completed with failures"
        },
    );
    report.update_metadata("type", "plan_execution");
    report.update_metadata("method", "plan_execution");
    report.update_metadata(
        "solver_tolerance",
        format!("{:.2e}", config.solver.tolerance()),
    );

    if !all_steps_validation_passed {
        let mut failures = Vec::new();
        for (step_id, r) in step_reports {
            if !r.validation_passed {
                failures.push(format!(
                    "{step_id}:{}",
                    r.validation_error.as_deref().unwrap_or("validation failed")
                ));
            }
        }
        report = report.with_validation_result(false, Some(failures.join("; ")));
    }

    if let Some(trace) = merge_step_traces(step_reports, config) {
        report = report.with_explanation(trace);
    }

    report
}

/// Perform "pre-flight" validation of a calibration step before execution.
///
/// This checks for quote availability, parameter consistency, and
/// cross-curve dependencies (e.g. valid discount curve for hazard calibration).
fn preflight_step(
    step: &crate::calibration::api::schema::CalibrationStep,
    quotes: &[crate::market::quotes::market_quote::MarketQuote],
    context: &MarketContext,
    _global_config: &crate::calibration::config::CalibrationConfig,
) -> Result<()> {
    match &step.params {
        StepParams::Discount(_p) => {
            let rates_quotes: Vec<crate::market::quotes::rates::RateQuote> =
                quotes.extract_quotes();
            if rates_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }
            let _curve_dc = curve_day_count_from_quotes(&rates_quotes)?;
            Ok(())
        }
        StepParams::Forward(_p) => {
            let rates_quotes: Vec<crate::market::quotes::rates::RateQuote> =
                quotes.extract_quotes();
            if rates_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            // Legacy validation removed (CalibrationPricer)
            // Legacy validation removed

            let _curve_dc = curve_day_count_from_quotes(&rates_quotes)?;
            Ok(())
        }
        StepParams::Hazard(p) => {
            // Ensure referenced discount curve exists.
            let _ = context.get_discount_ref(&p.discount_curve_id)?;

            if !p.notional.is_finite() || p.notional <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Hazard calibration notional must be positive; got {}",
                    p.notional
                )));
            }

            let cds_quotes: Vec<crate::market::quotes::cds::CdsQuote> = quotes.extract_quotes();
            if cds_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }
            for q in &cds_quotes {
                match q {
                    crate::market::quotes::cds::CdsQuote::CdsParSpread {
                        entity,
                        recovery_rate,
                        convention,
                        spread_bp,
                        ..
                    } => {
                        if *spread_bp <= 0.0 {
                            return Err(finstack_core::Error::Validation(format!(
                                "CDS spread_bp must be positive; got {}",
                                spread_bp
                            )));
                        }
                        if entity != &p.entity {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step entity mismatch: params.entity='{}' but quote.entity='{}'",
                                p.entity, entity
                            )));
                        }
                        if convention.currency != p.currency {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step currency mismatch: params.currency='{}' but quote.convention.currency='{}'",
                                p.currency, convention.currency
                            )));
                        }
                        if (recovery_rate - p.recovery_rate).abs() > 1e-12 {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step recovery mismatch: params.recovery_rate={} but quote.recovery_rate={}",
                                p.recovery_rate, recovery_rate
                            )));
                        }
                    }
                    crate::market::quotes::cds::CdsQuote::CdsUpfront {
                        entity,
                        recovery_rate,
                        convention,
                        running_spread_bp,
                        ..
                    } => {
                        if *running_spread_bp <= 0.0 {
                            return Err(finstack_core::Error::Validation(format!(
                                "CDS running_spread_bp must be positive; got {}",
                                running_spread_bp
                            )));
                        }
                        if entity != &p.entity {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step entity mismatch: params.entity='{}' but quote.entity='{}'",
                                p.entity, entity
                            )));
                        }
                        if convention.currency != p.currency {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step currency mismatch: params.currency='{}' but quote.convention.currency='{}'",
                                p.currency, convention.currency
                            )));
                        }
                        if (recovery_rate - p.recovery_rate).abs() > 1e-12 {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step recovery mismatch: params.recovery_rate={} but quote.recovery_rate={}",
                                p.recovery_rate, recovery_rate
                            )));
                        }
                    }
                }
            }
            Ok(())
        }
        StepParams::Inflation(p) => {
            let _ = context.get_discount_ref(&p.discount_curve_id)?;
            if !p.notional.is_finite() || p.notional <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Inflation calibration notional must be positive; got {}",
                    p.notional
                )));
            }
            if !p.base_cpi.is_finite() || p.base_cpi <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Inflation base_cpi must be positive; got {}",
                    p.base_cpi
                )));
            }

            // Validate observation lag string (used when no InflationIndex fixings are provided).
            let lag = p.observation_lag.trim();
            if !lag.is_empty() {
                let upper = lag.to_ascii_uppercase();
                let valid = upper == "NONE"
                    || upper == "0"
                    || upper == "0M"
                    || upper == "0D"
                    || upper
                        .strip_suffix('M')
                        .and_then(|n| n.trim().parse::<u8>().ok())
                        .is_some()
                    || upper
                        .strip_suffix('D')
                        .and_then(|n| n.trim().parse::<u16>().ok())
                        .is_some();
                if !valid {
                    return Err(finstack_core::Error::Validation(format!(
                        "Invalid observation_lag '{}': expected like '3M' or '90D'",
                        p.observation_lag
                    )));
                }
            }

            // If an InflationIndex fixings series is provided, enforce consistency:
            // - currency match
            // - lag match
            // - base CPI match (including any seasonality applied by the index)
            if let Some(index) = context.inflation_index_ref(p.curve_id.as_str()) {
                if index.currency != p.currency {
                    return Err(finstack_core::Error::Validation(format!(
                        "Inflation step currency mismatch: params.currency='{}' but InflationIndex.currency='{}'",
                        p.currency, index.currency
                    )));
                }

                // Parse observation lag and require it to match the index lag.
                let parsed_lag = {
                    let upper = p.observation_lag.trim().to_ascii_uppercase();
                    if upper == "NONE" || upper == "0" || upper == "0M" || upper == "0D" {
                        finstack_core::market_data::scalars::inflation_index::InflationLag::None
                    } else if let Some(num) = upper.strip_suffix('M') {
                        let months: u8 = num.trim().parse().map_err(|_| {
                            finstack_core::Error::Validation(format!(
                                "Invalid observation_lag '{}': expected like '3M'",
                                p.observation_lag
                            ))
                        })?;
                        finstack_core::market_data::scalars::inflation_index::InflationLag::Months(
                            months,
                        )
                    } else if let Some(num) = upper.strip_suffix('D') {
                        let days: u16 = num.trim().parse().map_err(|_| {
                            finstack_core::Error::Validation(format!(
                                "Invalid observation_lag '{}': expected like '90D'",
                                p.observation_lag
                            ))
                        })?;
                        finstack_core::market_data::scalars::inflation_index::InflationLag::Days(
                            days,
                        )
                    } else {
                        return Err(finstack_core::Error::Validation(format!(
                            "Invalid observation_lag '{}': expected like '3M' or '90D'",
                            p.observation_lag
                        )));
                    }
                };

                if parsed_lag != index.lag() {
                    return Err(finstack_core::Error::Validation(format!(
                        "Inflation step lag mismatch: params.observation_lag='{}' but InflationIndex.lag={:?}",
                        p.observation_lag,
                        index.lag()
                    )));
                }

                let expected_base = index.value_on(p.base_date).map_err(|e| {
                    finstack_core::Error::Validation(format!(
                        "Failed to resolve base CPI from InflationIndex '{}': {}",
                        p.curve_id.as_str(),
                        e
                    ))
                })?;
                let abs_tol = 1e-8_f64.max(1e-10_f64 * expected_base.abs());
                if (expected_base - p.base_cpi).abs() > abs_tol {
                    return Err(finstack_core::Error::Validation(format!(
                        "Inflation base_cpi mismatch: params.base_cpi={} but InflationIndex.value_on(base_date)={}",
                        p.base_cpi, expected_base
                    )));
                }
            }
            Ok(())
        }
        StepParams::VolSurface(p) => {
            let model = p.model.trim().to_ascii_lowercase();
            if model != "sabr" {
                return Err(finstack_core::Error::Validation(format!(
                    "VolSurface model '{}' is not supported (currently supported: 'sabr')",
                    p.model
                )));
            }
            let discount_id = p.discount_curve_id.as_deref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "VolSurface step requires discount_curve_id".to_string(),
                )
            })?;
            let _ = context.get_discount_ref(discount_id)?;
            Ok(())
        }
        StepParams::SwaptionVol(p) => {
            let _ = context.get_discount_ref(&p.discount_curve_id)?;
            if let crate::calibration::api::schema::SwaptionVolConvention::ShiftedLognormal {
                shift,
            } = p.vol_convention
            {
                if !shift.is_finite() || shift <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Shifted lognormal convention requires a finite, positive shift; got {}",
                        shift
                    )));
                }
            }
            Ok(())
        }
        StepParams::BaseCorrelation(p) => {
            if !p.notional.is_finite() || p.notional <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "BaseCorrelation calibration notional must be positive; got {}",
                    p.notional
                )));
            }

            // Base correlation calibration requires credit index data to be present in the context.
            let index_data = context.credit_index_ref(&p.index_id)?;

            // Market-standard: ensure recovery/currency/series/index are consistent.
            let tranche_quotes: Vec<crate::market::quotes::cds_tranche::CdsTrancheQuote> =
                quotes.extract_quotes();
            if tranche_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }
            let tranche_recovery: Option<f64> = None;

            for q in &tranche_quotes {
                match q {
                    crate::market::quotes::cds_tranche::CdsTrancheQuote::CDSTranche {
                        index,
                        attachment,
                        detachment,
                        convention,
                        ..
                    } => {
                        if index != &p.index_id {
                            continue;
                        }

                        if convention.currency != p.currency {
                            return Err(finstack_core::Error::Validation(format!(
                                "Base correlation tranche currency mismatch: params.currency='{}' but quote.convention.currency='{}'",
                                p.currency, convention.currency
                            )));
                        }

                        let normalize_pct = |value: f64| {
                            if (0.0..=1.0).contains(&value) {
                                value * 100.0
                            } else {
                                value
                            }
                        };
                        let attach_pct = normalize_pct(*attachment);
                        let detach_pct = normalize_pct(*detachment);
                        if !attach_pct.is_finite()
                            || !detach_pct.is_finite()
                            || attach_pct < 0.0
                            || !(0.0..=100.0).contains(&detach_pct)
                            || attach_pct >= detach_pct
                        {
                            return Err(finstack_core::Error::Validation(format!(
                                "Invalid tranche attachment/detachment: attachment={}, detachment={} (expect 0 <= attachment < detachment <= 100, percent or fraction)",
                                attachment, detachment
                            )));
                        }

                        // Note: CDS tranche quotes don't have recovery_rate in the convention.
                        // Recovery rate comes from the credit index data and is validated later.
                    }
                }
            }

            if let Some(r) = tranche_recovery {
                if (r - index_data.recovery_rate).abs() > 1e-12 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Tranche quote recovery_rate={} does not match credit index recovery_rate={}",
                        r, index_data.recovery_rate
                    )));
                }
            }

            Ok(())
        }
    }
}

enum OutputKey {
    Curve(CurveId),
    Surface(CurveId),
}

enum StepOutput {
    Curve(CurveStorage),
    Surface(Arc<VolSurface>),
}

struct StepBatchItem<'a> {
    step: &'a CalibrationStep,
    quotes: &'a [MarketQuote],
}

struct StepExecutionResult {
    output: StepOutput,
    credit_index_update: Option<(String, CreditIndexData)>,
    report: CalibrationReport,
}

fn base_correlation_curve_id(
    params: &crate::calibration::api::schema::BaseCorrelationParams,
) -> CurveId {
    CurveId::from(format!("{}_CORR", params.index_id))
}

fn step_output_key(step: &CalibrationStep) -> OutputKey {
    match &step.params {
        StepParams::Discount(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Forward(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Hazard(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::Inflation(p) => OutputKey::Curve(p.curve_id.clone()),
        StepParams::BaseCorrelation(p) => OutputKey::Curve(base_correlation_curve_id(p)),
        StepParams::VolSurface(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
        StepParams::SwaptionVol(p) => OutputKey::Surface(CurveId::from(p.surface_id.as_str())),
    }
}

fn extract_step_output(
    step: &CalibrationStep,
    context: &MarketContext,
) -> Result<(StepOutput, Option<(String, CreditIndexData)>)> {
    match &step.params {
        StepParams::Discount(p) => Ok((
            StepOutput::Curve(context.get_discount(&p.curve_id)?.into()),
            None,
        )),
        StepParams::Forward(p) => Ok((
            StepOutput::Curve(context.get_forward(&p.curve_id)?.into()),
            None,
        )),
        StepParams::Hazard(p) => Ok((
            StepOutput::Curve(context.get_hazard(&p.curve_id)?.into()),
            None,
        )),
        StepParams::Inflation(p) => Ok((
            StepOutput::Curve(context.get_inflation(&p.curve_id)?.into()),
            None,
        )),
        StepParams::BaseCorrelation(p) => {
            let curve_id = base_correlation_curve_id(p);
            let curve = context.get_base_correlation(curve_id.as_str())?;
            let credit_index_update = context
                .credit_index_ref(&p.index_id)
                .ok()
                .map(|idx| (p.index_id.clone(), idx.clone()));
            Ok((StepOutput::Curve(curve.into()), credit_index_update))
        }
        StepParams::VolSurface(p) => {
            Ok((StepOutput::Surface(context.surface(&p.surface_id)?), None))
        }
        StepParams::SwaptionVol(p) => {
            Ok((StepOutput::Surface(context.surface(&p.surface_id)?), None))
        }
    }
}

fn apply_step_output(
    context: &mut MarketContext,
    output: StepOutput,
    credit_index_update: Option<(String, CreditIndexData)>,
) {
    match output {
        StepOutput::Curve(curve) => {
            context.insert_mut(curve);
        }
        StepOutput::Surface(surface) => {
            context.insert_surface_mut(surface);
        }
    }

    if let Some((id, data)) = credit_index_update {
        context.insert_credit_index_mut(id, data);
    }
}

/// Execute a full [`CalibrationEnvelope`] plan.
///
/// This is the primary entry point for the calibration system. It
/// processes a sequential list of calibration steps, updates the market
/// context statefully, and produces a final aggregated result.
pub fn execute(envelope: &CalibrationEnvelope) -> Result<CalibrationResultEnvelope> {
    let mut context: MarketContext = match &envelope.initial_market {
        Some(state) => MarketContext::try_from(state.clone())
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?,
        None => MarketContext::new(),
    };
    let plan = &envelope.plan;
    let mut aggregated_residuals = BTreeMap::new();
    let mut total_iterations = 0;
    let mut step_reports = BTreeMap::new();

    // 1. Execution loop (with per-step preflight validation against the current context)
    if plan.settings.use_parallel {
        let mut index = 0;
        while index < plan.steps.len() {
            let mut batch = Vec::new();
            let mut curve_outputs = HashSet::new();
            let mut surface_outputs = HashSet::new();

            while index < plan.steps.len() {
                let step = &plan.steps[index];
                let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
                    finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                        id: format!("Quote set '{}' not found", step.quote_set),
                    })
                })?;

                match preflight_step(step, quotes, &context, &plan.settings) {
                    Ok(()) => {}
                    Err(err) => {
                        if batch.is_empty() {
                            return Err(err);
                        }
                        break;
                    }
                }

                match step_output_key(step) {
                    OutputKey::Curve(id) => {
                        if !curve_outputs.insert(id) {
                            break;
                        }
                    }
                    OutputKey::Surface(id) => {
                        if !surface_outputs.insert(id) {
                            break;
                        }
                    }
                }

                batch.push(StepBatchItem {
                    step,
                    quotes: quotes.as_slice(),
                });
                index += 1;
            }

            let results: Vec<StepExecutionResult> = if batch.len() == 1 {
                let item = &batch[0];
                let (new_context, report) =
                    execute_step(&item.step.params, item.quotes, &context, &plan.settings)?;
                let (output, credit_index_update) = extract_step_output(item.step, &new_context)?;
                vec![StepExecutionResult {
                    output,
                    credit_index_update,
                    report,
                }]
            } else {
                batch
                    .par_iter()
                    .map(|item| {
                        let (new_context, report) =
                            execute_step(&item.step.params, item.quotes, &context, &plan.settings)?;
                        let (output, credit_index_update) =
                            extract_step_output(item.step, &new_context)?;
                        Ok(StepExecutionResult {
                            output,
                            credit_index_update,
                            report,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            };

            for (item, result) in batch.iter().zip(results) {
                apply_step_output(&mut context, result.output, result.credit_index_update);

                for (k, v) in &result.report.residuals {
                    aggregated_residuals.insert(format!("{}:{}", item.step.id, k), *v);
                }
                total_iterations += result.report.iterations;
                step_reports.insert(item.step.id.clone(), result.report);
            }
        }
    } else {
        for step in &plan.steps {
            let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
                finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                    id: format!("Quote set '{}' not found", step.quote_set),
                })
            })?;

            preflight_step(step, quotes, &context, &plan.settings)?;

            let (new_context, report) =
                execute_step(&step.params, quotes, &context, &plan.settings)?;

            context = new_context;

            // Aggregate report
            for (k, v) in &report.residuals {
                aggregated_residuals.insert(format!("{}:{}", step.id, k), *v);
            }
            total_iterations += report.iterations;
            step_reports.insert(step.id.clone(), report);
        }
    }

    // 2. Build result
    let aggregated_report = aggregate_plan_report(
        aggregated_residuals,
        total_iterations,
        &step_reports,
        &plan.settings,
    );

    let result = CalibrationResult {
        final_market: (&context).into(),
        report: aggregated_report,
        step_reports,
        results_meta: finstack_core::config::results_meta(
            &finstack_core::config::FinstackConfig::default(),
        ),
    };

    Ok(CalibrationResultEnvelope::new(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::explain::ExplanationTrace;

    #[test]
    fn aggregated_report_computes_rmse_and_objective() {
        let mut step_reports = BTreeMap::new();
        step_reports.insert(
            "s1".to_string(),
            CalibrationReport::new(BTreeMap::from([("a".to_string(), 3.0)]), 2, true, "ok"),
        );
        step_reports.insert(
            "s2".to_string(),
            CalibrationReport::new(BTreeMap::from([("b".to_string(), 4.0)]), 3, true, "ok"),
        );

        let aggregated_residuals =
            BTreeMap::from([("s1:a".to_string(), 3.0), ("s2:b".to_string(), 4.0)]);
        let cfg = crate::calibration::config::CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default().with_tolerance(1e-12),
            ..Default::default()
        };
        let report = aggregate_plan_report(aggregated_residuals, 5, &step_reports, &cfg);

        let expected = ((3.0_f64 * 3.0 + 4.0 * 4.0) / 2.0).sqrt();
        assert!((report.rmse - expected).abs() < 1e-12);
        assert!((report.objective_value - expected).abs() < 1e-12);
    }

    #[test]
    fn aggregated_report_merges_step_traces_when_enabled() {
        let mut step_reports = BTreeMap::new();
        let mut r1 = CalibrationReport::new(BTreeMap::new(), 0, true, "ok");
        r1.explanation = Some(ExplanationTrace {
            trace_type: "calibration".to_string(),
            entries: vec![TraceEntry::ComputationStep {
                name: "inner".to_string(),
                description: "inner step".to_string(),
                metadata: None,
            }],
            truncated: None,
        });
        step_reports.insert("s1".to_string(), r1);

        let cfg = crate::calibration::config::CalibrationConfig {
            explain: finstack_core::explain::ExplainOpts::enabled(),
            ..Default::default()
        };
        let report = aggregate_plan_report(BTreeMap::new(), 0, &step_reports, &cfg);
        let trace = report.explanation.expect("merged explanation");
        assert!(
            trace
                .entries
                .iter()
                .any(|e| matches!(e, TraceEntry::ComputationStep { name, .. } if name == "inner")),
            "expected merged trace to contain the step's entries"
        );
    }
}

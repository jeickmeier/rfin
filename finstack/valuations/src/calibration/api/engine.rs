//! Calibration execution engine.
//!
//! Orchestrates the execution of a calibration plan.

use super::schema::CalibrationEnvelope;
use crate::calibration::api::schema::CalibrationStep;
use crate::calibration::api::schema::{CalibrationResult, CalibrationResultEnvelope};
use crate::calibration::step_runtime;
use crate::calibration::step_runtime::{OutputKey, StepOutcome};
use crate::calibration::validation::preflight_step;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashSet};

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

struct StepBatchItem<'a> {
    step: &'a CalibrationStep,
    quotes: &'a [MarketQuote],
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
            let mut curve_outputs: finstack_core::HashSet<_> = HashSet::default();
            let mut surface_outputs: finstack_core::HashSet<_> = HashSet::default();

            while index < plan.steps.len() {
                let step = &plan.steps[index];
                let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
                    finstack_core::Error::Input(finstack_core::InputError::NotFound {
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

                match step_runtime::output_key(step) {
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

            let results: Vec<StepOutcome> = if batch.len() == 1 {
                let item = &batch[0];
                let outcome =
                    step_runtime::execute(item.step, item.quotes, &context, &plan.settings)?;
                vec![outcome]
            } else {
                batch
                    .par_iter()
                    .map(|item| {
                        step_runtime::execute(item.step, item.quotes, &context, &plan.settings)
                    })
                    .collect::<Result<Vec<_>>>()?
            };

            for (item, result) in batch.iter().zip(results) {
                step_runtime::apply_output(&mut context, result.output, result.credit_index_update);

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
                finstack_core::Error::Input(finstack_core::InputError::NotFound {
                    id: format!("Quote set '{}' not found", step.quote_set),
                })
            })?;

            preflight_step(step, quotes, &context, &plan.settings)?;

            let outcome = step_runtime::execute(step, quotes, &context, &plan.settings)?;
            step_runtime::apply_output(&mut context, outcome.output, outcome.credit_index_update);

            // Aggregate report
            for (k, v) in &outcome.report.residuals {
                aggregated_residuals.insert(format!("{}:{}", step.id, k), *v);
            }
            total_iterations += outcome.report.iterations;
            step_reports.insert(step.id.clone(), outcome.report);
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
#[allow(clippy::expect_used, clippy::panic)]
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

    #[test]
    fn aggregated_report_surfaces_validation_failures() {
        let mut step_reports = BTreeMap::new();
        let failed = CalibrationReport::new(BTreeMap::new(), 1, true, "converged")
            .with_validation_result(false, Some("invalid curve shape".to_string()));
        step_reports.insert("curve_step".to_string(), failed);

        let cfg = crate::calibration::config::CalibrationConfig::default();
        let report = aggregate_plan_report(BTreeMap::new(), 1, &step_reports, &cfg);

        assert!(!report.validation_passed);
        assert!(!report.success);
        let msg = report
            .validation_error
            .as_deref()
            .expect("validation error should be present");
        assert!(
            msg.contains("curve_step:invalid curve shape"),
            "expected step id and reason in validation error: {msg}"
        );
    }
}

//! Calibration execution engine.
//!
//! Orchestrates the execution of a calibration plan.

use super::schema::CalibrationEnvelopeV2;
use crate::calibration::v2::adapters::handlers::execute_step;
use crate::calibration::{CalibrationReport, CalibrationResult, CalibrationResultEnvelope};
use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;
use std::collections::BTreeMap;

/// Execute a calibration plan.
pub fn execute(envelope: &CalibrationEnvelopeV2) -> Result<CalibrationResultEnvelope> {
    let mut context: MarketContext = match &envelope.initial_market {
        Some(state) => MarketContext::try_from(state.clone())
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?,
        None => MarketContext::new(),
    };
    let plan = &envelope.plan;
    let mut aggregated_residuals = BTreeMap::new();
    let mut total_iterations = 0;
    let mut step_reports = BTreeMap::new();

    // 1. Preflight validation (optional)
    // Could check dependencies here.

    // 2. Execution loop
    for step in &plan.steps {
        let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: format!("Quote set '{}' not found", step.quote_set),
            })
        })?;

        let (new_context, report) = execute_step(&step.params, quotes, &context, &plan.settings)?;
        
        context = new_context;
        
        // Aggregate report
        for (k, v) in &report.residuals {
            aggregated_residuals.insert(format!("{}:{}", step.id, k), *v);
        }
        total_iterations += report.iterations;
        step_reports.insert(step.id.clone(), report);
    }

    // 3. Build result
    // Combine residuals and metadata
    let aggregated_report = CalibrationReport {
        success: step_reports.values().all(|r| r.success),
        // method field removed, put in metadata
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("method".to_string(), "plan_execution".to_string());
            m
        },
        max_residual: aggregated_residuals.values().cloned().fold(0.0_f64, f64::max),
        iterations: total_iterations,
        residuals: aggregated_residuals,
        explanation: None, // Could merge traces if needed
        validation_passed: true,
        validation_error: None,
        rmse: 0.0, // Should calculate RMSE
        objective_value: 0.0,
        convergence_reason: "Plan execution completed".to_string(),
        solver_config: Default::default(),
        results_meta: Default::default(),
    };

    let result = CalibrationResult {
        final_market: (&context).into(),
        report: aggregated_report,
        step_reports,
        results_meta: Default::default(),
    };

    Ok(CalibrationResultEnvelope::new(result))
}

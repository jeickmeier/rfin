//! Calibration execution engine.
//!
//! Orchestrates the execution of a calibration plan.

use super::schema::CalibrationEnvelopeV2;
use crate::calibration::adapters::handlers::discount_curve_day_count;
use crate::calibration::adapters::handlers::{apply_rates_step_conventions, execute_step};
use crate::calibration::api::schema::StepParams;
use crate::calibration::api::schema::{CalibrationResult, CalibrationResultEnvelope};
use crate::calibration::pricing::{CalibrationPricer, RatesQuoteUseCase};
use crate::calibration::quotes::ExtractQuotes;
use crate::calibration::CalibrationReport;
use finstack_core::explain::{ExplanationTrace, TraceEntry};
use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;
use std::collections::BTreeMap;

fn merge_step_traces(
    step_reports: &BTreeMap<String, CalibrationReport>,
    config: &crate::calibration::config::CalibrationConfig,
) -> Option<ExplanationTrace> {
    if !config.explain.enabled {
        return None;
    }

    let mut merged = ExplanationTrace::new("calibration_v2_plan");
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

fn preflight_step(
    step: &crate::calibration::api::schema::CalibrationStepV2,
    quotes: &[crate::calibration::quotes::MarketQuote],
    context: &MarketContext,
    global_config: &crate::calibration::config::CalibrationConfig,
) -> Result<()> {
    match &step.params {
        StepParams::Discount(p) => {
            let rates_quotes = quotes.extract_quotes();
            if rates_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            let pricer = CalibrationPricer::new(p.base_date, p.curve_id.clone())
                .with_discount_curve_id(p.pricing_discount_id.clone().unwrap_or(p.curve_id.clone()))
                .with_forward_curve_id(p.pricing_forward_id.clone().unwrap_or(p.curve_id.clone()));
            let pricer = apply_rates_step_conventions(pricer, p.currency, &p.conventions, true)?;

            // Quote validation + curve dependency checks before solving.
            let bounds = global_config.effective_rate_bounds(p.currency);
            CalibrationPricer::validate_rates_quotes(
                &rates_quotes,
                &bounds,
                p.base_date,
                RatesQuoteUseCase::DiscountCurve {
                    enforce_separation: p.conventions.enforce_discount_separation.unwrap_or(false),
                },
            )?;
            pricer.validate_curve_dependencies(&rates_quotes, context)?;

            let _curve_dc = discount_curve_day_count(p.currency, &p.conventions);
            Ok(())
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
            );
            let pricer = apply_rates_step_conventions(pricer, p.currency, &p.conventions, false)?;

            let bounds = global_config.effective_rate_bounds(p.currency);
            CalibrationPricer::validate_rates_quotes(
                &rates_quotes,
                &bounds,
                p.base_date,
                RatesQuoteUseCase::ForwardCurve,
            )?;
            pricer.validate_curve_dependencies(&rates_quotes, context)?;

            let _curve_dc = discount_curve_day_count(p.currency, &p.conventions);
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

            // Market-standard: ensure recovery/currency/entity are consistent between params and quotes.
            let credit_quotes = quotes.extract_quotes();
            if credit_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }
            for q in &credit_quotes {
                match q {
                    crate::calibration::quotes::CreditQuote::CDS {
                        entity,
                        recovery_rate,
                        currency,
                        spread_bp,
                        conventions,
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
                        if currency != &p.currency {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step currency mismatch: params.currency='{}' but quote.currency='{}'",
                                p.currency, currency
                            )));
                        }
                        if (recovery_rate - p.recovery_rate).abs() > 1e-12 {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step recovery mismatch: params.recovery_rate={} but quote.recovery_rate={}",
                                p.recovery_rate, recovery_rate
                            )));
                        }
                        if let Some(r) = conventions.recovery_rate {
                            if (r - p.recovery_rate).abs() > 1e-12 {
                                return Err(finstack_core::Error::Validation(format!(
                                    "Hazard step recovery mismatch: params.recovery_rate={} but quote.conventions.recovery_rate={}",
                                    p.recovery_rate, r
                                )));
                            }
                        }
                        if let Some(c) = conventions.currency {
                            if c != p.currency {
                                return Err(finstack_core::Error::Validation(format!(
                                    "Hazard step currency mismatch: params.currency='{}' but quote.conventions.currency='{}'",
                                    p.currency, c
                                )));
                            }
                        }
                    }
                    crate::calibration::quotes::CreditQuote::CDSUpfront {
                        entity,
                        recovery_rate,
                        currency,
                        running_spread_bp,
                        conventions,
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
                        if currency != &p.currency {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step currency mismatch: params.currency='{}' but quote.currency='{}'",
                                p.currency, currency
                            )));
                        }
                        if (recovery_rate - p.recovery_rate).abs() > 1e-12 {
                            return Err(finstack_core::Error::Validation(format!(
                                "Hazard step recovery mismatch: params.recovery_rate={} but quote.recovery_rate={}",
                                p.recovery_rate, recovery_rate
                            )));
                        }
                        if let Some(r) = conventions.recovery_rate {
                            if (r - p.recovery_rate).abs() > 1e-12 {
                                return Err(finstack_core::Error::Validation(format!(
                                    "Hazard step recovery mismatch: params.recovery_rate={} but quote.conventions.recovery_rate={}",
                                    p.recovery_rate, r
                                )));
                            }
                        }
                        if let Some(c) = conventions.currency {
                            if c != p.currency {
                                return Err(finstack_core::Error::Validation(format!(
                                    "Hazard step currency mismatch: params.currency='{}' but quote.conventions.currency='{}'",
                                    p.currency, c
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(finstack_core::Error::Input(
                            finstack_core::error::InputError::Invalid,
                        ))
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
            if p.discount_curve_id.is_none() {
                return Err(finstack_core::Error::Validation(
                    "VolSurface step requires discount_curve_id".to_string(),
                ));
            }
            let _ = context.get_discount_ref(p.discount_curve_id.as_ref().expect("checked"))?;
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
            let _ = context.get_discount_ref(&p.discount_curve_id)?;
            let index_data = context.credit_index_ref(&p.index_id)?;

            if !p.notional.is_finite() || p.notional <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Base correlation calibration notional must be positive; got {}",
                    p.notional
                )));
            }

            let credit_quotes: Vec<crate::calibration::quotes::CreditQuote> =
                quotes.extract_quotes();
            if credit_quotes.is_empty() {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::TooFewPoints,
                ));
            }

            // Recovery consistency (if explicitly provided on tranche quotes).
            let mut tranche_recovery: Option<f64> = None;

            for q in &credit_quotes {
                if let crate::calibration::quotes::CreditQuote::CDSTranche {
                    index,
                    attachment,
                    detachment,
                    conventions,
                    ..
                } = q
                {
                    if index != &p.index_id {
                        continue;
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

                    if conventions.payment_frequency.is_none() && p.payment_frequency.is_none() {
                        return Err(finstack_core::Error::Validation(
                            "Missing tranche payment frequency; set quote.conventions.payment_frequency or params.payment_frequency"
                                .to_string(),
                        ));
                    }
                    if conventions.day_count.is_none() && p.day_count.is_none() {
                        return Err(finstack_core::Error::Validation(
                            "Missing tranche day count; set quote.conventions.day_count or params.day_count"
                                .to_string(),
                        ));
                    }
                    if conventions.business_day_convention.is_none()
                        && p.business_day_convention.is_none()
                    {
                        return Err(finstack_core::Error::Validation(
                            "Missing tranche business day convention; set quote.conventions.business_day_convention or params.business_day_convention"
                                .to_string(),
                        ));
                    }

                    if let Some(r) = conventions.recovery_rate {
                        if let Some(prev) = tranche_recovery {
                            if (r - prev).abs() > 1e-12 {
                                return Err(finstack_core::Error::Validation(format!(
                                    "Inconsistent tranche quote recovery rates: {} vs {}",
                                    prev, r
                                )));
                            }
                        }
                        tranche_recovery = Some(r);
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

    // 1. Execution loop (with per-step preflight validation against the current context)
    for step in &plan.steps {
        let quotes = plan.quote_sets.get(&step.quote_set).ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: format!("Quote set '{}' not found", step.quote_set),
            })
        })?;

        preflight_step(step, quotes, &context, &plan.settings)?;

        let (new_context, report) = execute_step(&step.params, quotes, &context, &plan.settings)?;

        context = new_context;

        // Aggregate report
        for (k, v) in &report.residuals {
            aggregated_residuals.insert(format!("{}:{}", step.id, k), *v);
        }
        total_iterations += report.iterations;
        step_reports.insert(step.id.clone(), report);
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
            trace_type: "calibration_v2".to_string(),
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

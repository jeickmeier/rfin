//! Statistical and data-collection helpers for formula evaluation.
//!
//! Pure functions that operate on evaluation contexts to gather, sort, and
//! reduce historical node values. These have no dependency on the expression
//! evaluator, so they can be shared by both `formula.rs` and
//! `formula_aggregates.rs` without creating cycles.

use crate::error::Result;
use crate::evaluator::context::EvaluationContext;
use finstack_core::dates::PeriodId;
use finstack_core::math::kahan_sum;
use std::collections::BTreeMap;

/// Decode an internal `__cs__<component>__<instrument>` reference.
pub(crate) fn decode_cs_reference(name: &str) -> Option<(&str, &str)> {
    if name.starts_with("__cs__") {
        let parts: Vec<&str> = name.split("__").collect();
        if parts.len() == 4 && parts[0].is_empty() && parts[1] == "cs" {
            return Some((parts[2], parts[3]));
        }
    }
    None
}

/// Get a single historical value for a node or cs-reference at a target period.
pub(crate) fn get_historical_column_value(
    context: &EvaluationContext,
    node_name: &str,
    target_period: &PeriodId,
) -> Option<f64> {
    if let Some((component, instrument_or_total)) = decode_cs_reference(node_name) {
        context
            .get_historical_cs_value(component, instrument_or_total, target_period)
            .ok()
    } else {
        context.get_historical_value(node_name, target_period)
    }
}

/// Collect historical values sorted chronologically.
///
/// Returns a BTreeMap of period -> value for all historical periods plus current.
/// This is a common helper used by rolling window and statistical functions.
pub(crate) fn collect_historical_values_sorted(
    node_name: &str,
    context: &EvaluationContext,
) -> Result<BTreeMap<PeriodId, f64>> {
    if let Some((component, instrument_or_total)) = decode_cs_reference(node_name) {
        let mut sorted_periods = BTreeMap::new();
        for period in context.historical_capital_structure_cashflows.keys() {
            if let Ok(value) =
                context.get_historical_cs_value(component, instrument_or_total, period)
            {
                sorted_periods.insert(*period, value);
            }
        }
        if let Ok(current) = context.get_cs_value(component, instrument_or_total) {
            sorted_periods.insert(context.period_id, current);
        }
        return Ok(sorted_periods);
    }

    let mut sorted_periods = BTreeMap::new();

    for (period, values) in context.historical_results.iter() {
        if let Some(value) = values.get(node_name) {
            sorted_periods.insert(*period, *value);
        }
    }

    if let Ok(current) = context.get_value(node_name) {
        sorted_periods.insert(context.period_id, current);
    }

    Ok(sorted_periods)
}

/// Collect values for a rolling window in chronological order.
/// Returns values from oldest to newest within the window.
pub(crate) fn collect_rolling_window_values(
    node_name: &str,
    context: &EvaluationContext,
    window_size: usize,
) -> Result<Vec<f64>> {
    if window_size == 0 {
        return Ok(Vec::new());
    }

    let sorted = collect_historical_values_sorted(node_name, context)?;

    let mut values: Vec<f64> = sorted.into_values().rev().take(window_size).collect();
    values.reverse();

    Ok(values)
}

/// Collect all historical values for a node including current.
pub(crate) fn collect_all_historical_values(
    node_name: &str,
    context: &EvaluationContext,
) -> Result<Vec<f64>> {
    let sorted = collect_historical_values_sorted(node_name, context)?;
    Ok(sorted.into_values().collect())
}

/// Collect values for a node over a closed period range [start, end].
///
/// Periods are compared using their natural ordering. Values are returned in
/// chronological order (oldest -> newest).
pub(crate) fn collect_period_range_values(
    node_name: &str,
    context: &EvaluationContext,
    start: PeriodId,
    end: PeriodId,
) -> Result<Vec<f64>> {
    let sorted = collect_historical_values_sorted(node_name, context)?;
    Ok(sorted
        .into_iter()
        .filter(|(period, _)| *period >= start && *period <= end)
        .map(|(_, value)| value)
        .collect())
}

/// Calculate mean of values.
pub(crate) fn calculate_mean(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    Ok(kahan_sum(values.iter().copied()) / values.len() as f64)
}

/// Calculate standard deviation of values.
///
/// Uses sample standard deviation (sqrt of sample variance) per financial industry standards.
pub(crate) fn calculate_std(values: &[f64]) -> Result<f64> {
    if values.len() < 2 {
        return Ok(f64::NAN);
    }
    let variance = calculate_variance(values)?;
    Ok(variance.sqrt())
}

/// Calculate variance of values.
///
/// Uses sample variance (Bessel's correction with n-1 denominator) per financial industry standards.
/// This is the unbiased estimator required by Bloomberg, Excel VAR.S(), pandas.var(ddof=1), etc.
pub(crate) fn calculate_variance(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    if values.len() == 1 {
        return Ok(f64::NAN);
    }
    let mean = calculate_mean(values)?;
    let squared_diffs = values.iter().map(|v| (v - mean).powi(2));
    Ok(kahan_sum(squared_diffs) / (values.len() - 1) as f64)
}

/// Calculate median of values.
pub(crate) fn calculate_median(values: &[f64]) -> Result<f64> {
    if values.is_empty() {
        return Ok(f64::NAN);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    if len.is_multiple_of(2) {
        Ok((sorted[len / 2 - 1] + sorted[len / 2]) / 2.0)
    } else {
        Ok(sorted[len / 2])
    }
}

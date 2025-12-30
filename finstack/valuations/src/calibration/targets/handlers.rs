//! Calibration handlers (targets).
//!
//! Maps API steps to domain logic execution.

use crate::calibration::api::schema::CalibrationStep;
use crate::calibration::api::schema::StepParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::step_runtime;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Execute a single calibration step.
///
/// This is the main entry point for the calibration engine to process a
/// [`CalibrationStep`]. It extracts relevant quotes, prepares the
/// target target, executes the solver (Bootstrap or Global), and
/// updates the market context with the result.
///
/// # Returns
///
/// A tuple containing:
/// - Updated [`MarketContext`] with the new curve/surface.
/// - [`CalibrationReport`] containing solver performance and diagnostics.
pub(crate) fn execute_step(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    let dummy_step = CalibrationStep {
        id: "step".to_string(),
        quote_set: "quotes".to_string(),
        params: params.clone(),
    };
    let outcome = step_runtime::execute(&dummy_step, quotes, context, global_config)?;
    let mut new_context = context.clone();
    step_runtime::apply_output(
        &mut new_context,
        outcome.output,
        outcome.credit_index_update,
    );
    Ok((new_context, outcome.report))
}

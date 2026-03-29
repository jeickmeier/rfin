//! PIK toggle evaluation, target scoping, and hysteresis.

use crate::capital_structure::state::CapitalStructureState;
use crate::capital_structure::waterfall_spec::PikToggleSpec;
use crate::error::Result;
use crate::evaluator::EvaluationContext;

use super::eval_value_or_formula;

/// Evaluate PIK toggle conditions and return whether PIK should be enabled.
pub(super) fn evaluate_pik_toggle(
    context: &EvaluationContext,
    pik_spec: &PikToggleSpec,
) -> Result<bool> {
    let metric_value = eval_value_or_formula(context, &pik_spec.liquidity_metric)?;
    Ok(metric_value < pik_spec.threshold)
}

/// Check whether PIK mode is currently active for an instrument.
pub(super) fn is_pik_enabled(state: &CapitalStructureState, instrument_id: &str) -> bool {
    state.pik_mode.get(instrument_id).copied().unwrap_or(false)
}

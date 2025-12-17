//! Calibration plan bumping utilities.

use crate::calibration::v2::api::schema::CalibrationPlanV2;

/// Utilities for bumping calibration plans (risk analysis).
pub struct PlanBumper;

impl PlanBumper {
    /// Apply a parallel bump to all quotes in the plan.
    ///
    /// This modifies the plan in-place.
    ///
    /// # Arguments
    ///
    /// * `plan` - The calibration plan to modify
    /// * `amount` - Bump amount, interpreted per quote type (see `MarketQuote::bump`)
    pub fn bump_parallel(plan: &mut CalibrationPlanV2, amount: f64) {
        for quotes in plan.quote_sets.values_mut() {
            for quote in quotes.iter_mut() {
                *quote = quote.bump(amount);
            }
        }
    }

    /// Create a new plan with parallel bump applied.
    pub fn apply_parallel_bump(mut plan: CalibrationPlanV2, amount: f64) -> CalibrationPlanV2 {
        Self::bump_parallel(&mut plan, amount);
        plan
    }
}

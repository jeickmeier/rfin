//! Weighted average cost metric for revolving credit facilities.

use crate::instruments::RevolvingCredit;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculator for weighted average cost across all fees and interest.
///
/// Computes the effective annualized cost combining:
/// - Base interest rate on drawn amounts
/// - Commitment fee on undrawn
/// - Usage fee on drawn
/// - Facility fee on total commitment
///
/// Returns the weighted average as a rate (decimal).
#[derive(Debug, Default, Clone, Copy)]
pub struct WeightedAverageCostCalculator;

impl MetricCalculator for WeightedAverageCostCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;

        // Get base interest rate
        let base_rate = match &facility.base_rate_spec {
            crate::instruments::revolving_credit::types::BaseRateSpec::Fixed { rate } => *rate,
            crate::instruments::revolving_credit::types::BaseRateSpec::Floating {
                index_id,
                margin_bp,
                ..
            } => {
                // Use forward curve to get current rate
                let fwd = context.curves.get_forward_ref(index_id.as_str())?;
                let index_rate = fwd.rate(0.25); // Use 3M as representative
                index_rate + (margin_bp * 1e-4)
            }
        };

        let commitment_amount = facility.commitment_amount.amount();

        if commitment_amount == 0.0 {
            return Ok(0.0);
        }

        // Calculate total annual cost
        let drawn_amt = facility.drawn_amount.amount();
        let undrawn_amt = commitment_amount - drawn_amt;

        // Interest on drawn
        let interest_cost = drawn_amt * base_rate;

        // Commitment fee on undrawn
        let commitment_cost = undrawn_amt * (facility.fees.commitment_fee_bp * 1e-4);

        // Usage fee on drawn
        let usage_cost = drawn_amt * (facility.fees.usage_fee_bp * 1e-4);

        // Facility fee on total commitment
        let facility_cost = commitment_amount * (facility.fees.facility_fee_bp * 1e-4);

        // Total annual cost
        let total_cost = interest_cost + commitment_cost + usage_cost + facility_cost;

        // Weighted average as a percentage of commitment
        let weighted_avg_cost = total_cost / commitment_amount;

        Ok(weighted_avg_cost)
    }
}

//! Theta calculator for interest rate options.
//!
//! Computes theta via a bump-and-reprice approach: reprice the instrument
//! at `as_of + period` (default 1D) holding market curves and vol surface fixed.

use crate::instruments::cap_floor::InterestRateOption;
use crate::instruments::common::metrics::theta_utils;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Theta calculator (bump-and-reprice with customizable period)
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Get theta period from pricing overrides, default to "1D"
        let period_str = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.theta_period.as_deref())
            .unwrap_or("1D");

        // Calculate rolled date (capping at instrument expiry)
        let expiry_date = Some(option.end_date);
        let rolled_date =
            theta_utils::calculate_theta_date(context.as_of, period_str, expiry_date)?;

        // If already expired or rolling to same date, theta is zero
        if rolled_date <= context.as_of {
            return Ok(0.0);
        }

        // Base PV from context
        let base_pv = context.base_value.amount();

        // Reprice at rolled date with same market context
        let bumped = option.npv(&context.curves, rolled_date)?;

        Ok(bumped.amount() - base_pv)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

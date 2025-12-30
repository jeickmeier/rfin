//! Expected Loss metric for CDS Index.
//!
//! Calculates the expected loss at maturity for a CDS index using:
//! EL = Notional × PD × LGD
//!
//! For a CDS index, we use the index-level hazard curve and average recovery rate,
//! which represents the expected loss across all constituents assuming equal weighting.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Expected Loss calculator for CDS Index.
pub struct ExpectedLossCalculator;

impl MetricCalculator for ExpectedLossCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        // Get hazard curve for the index from protection leg
        let hazard = context
            .curves
            .get_hazard(index.protection.credit_curve_id.as_str())?;
        let base_date = hazard.base_date();

        // Calculate time to maturity in years
        let t_maturity = index.premium.dc.year_fraction(
            base_date,
            index.premium.end,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Survival probability to maturity (index-level)
        let survival_prob = hazard.sp(t_maturity);

        // Default probability
        let default_prob = 1.0 - survival_prob;

        // Loss given default (using index average recovery from protection leg)
        let lgd = 1.0 - index.protection.recovery_rate;

        // Expected loss on index notional
        // Note: This is a simplified approach assuming the index hazard curve
        // already reflects the weighted average default risk of constituents
        let expected_loss = index.notional.amount() * default_prob * lgd;

        Ok(expected_loss)
    }
}

//! Expected Loss metric for CDS Index.
//!
//! Calculates the expected loss at maturity for a CDS index using:
//! EL = Notional × PD × LGD
//!
//! For a CDS index, we use the index-level hazard curve and average recovery rate,
//! which represents the expected loss across all constituents assuming equal weighting.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;
use finstack_core::Result;

/// Expected Loss calculator for CDS Index.
pub struct ExpectedLossCalculator;

impl MetricCalculator for ExpectedLossCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        let as_of = context.as_of;

        // Get hazard curve for the index from protection leg
        let hazard = context
            .curves
            .get_hazard(index.protection.credit_curve_id.as_str())?;

        // If already at/after maturity, no forward expected loss.
        if as_of >= index.premium.end {
            return Ok(0.0);
        }

        // Use hazard curve's day-count for survival time axis.
        let dc = hazard.day_count();
        let base_date = hazard.base_date();
        let t_asof = dc.year_fraction(base_date, as_of, DayCountCtx::default())?;
        let t_maturity = dc.year_fraction(base_date, index.premium.end, DayCountCtx::default())?;

        // Conditional survival to maturity given survival to as_of.
        let sp_asof = hazard.sp(t_asof);
        let sp_maturity = hazard.sp(t_maturity);
        let survival_cond = if sp_asof > 0.0 {
            sp_maturity / sp_asof
        } else {
            0.0
        };

        // Default probability
        let default_prob = (1.0 - survival_cond).clamp(0.0, 1.0);

        // Loss given default (using index average recovery from protection leg)
        let lgd = 1.0 - index.protection.recovery_rate;

        // Expected loss on index notional
        // Note: This is a simplified approach assuming the index hazard curve
        // already reflects the weighted average default risk of constituents
        let scale = index.index_factor;
        let expected_loss = index.notional.amount() * scale * default_prob * lgd;

        Ok(expected_loss)
    }
}

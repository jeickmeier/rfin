//! Expected Loss metric for single-name CDS.
//!
//! Calculates the expected loss at maturity using the market-standard formula:
//! EL = Notional × PD × LGD
//!
//! Where:
//! - PD (Probability of Default) = 1 - Survival Probability to maturity
//! - LGD (Loss Given Default) = 1 - Recovery Rate

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Expected Loss calculator for single-name CDS.
pub struct ExpectedLossCalculator;

impl MetricCalculator for ExpectedLossCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;

        // Get hazard curve from protection leg
        let hazard = context
            .curves
            .get_hazard_ref(cds.protection.credit_curve_id.as_str())?;
        let base_date = hazard.base_date();

        // Calculate time to maturity in years
        let t_maturity = cds.premium.dc.year_fraction(
            base_date,
            cds.premium.end,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Survival probability to maturity
        let survival_prob = hazard.sp(t_maturity);

        // Default probability
        let default_prob = 1.0 - survival_prob;

        // Loss given default
        let lgd = 1.0 - cds.protection.recovery_rate;

        // Expected loss in currency units
        let expected_loss = cds.notional.amount() * default_prob * lgd;

        Ok(expected_loss)
    }
}

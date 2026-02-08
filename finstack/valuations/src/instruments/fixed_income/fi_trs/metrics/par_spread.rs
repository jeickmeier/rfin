//! Par spread calculator for fixed income index TRS.

use crate::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result};

/// Calculates the par spread for a fixed income index TRS (spread that makes NPV = 0).
///
/// The par spread is the spread over the floating rate that makes the net present value
/// of the TRS equal to zero. This is calculated as the ratio of the total return leg PV
/// to the financing annuity, scaled to basis points.
///
/// Computed from the total-return-receiver's perspective regardless of trade side,
/// since par spread is a market-level quote (analogous to a swap rate).
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::FinancingAnnuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let trs: &FIIndexTotalReturnSwap = context.instrument_as()?;
        let curves = context.curves.as_ref();
        let as_of = context.as_of;

        // Get financing annuity
        let annuity = trs.financing_annuity(curves, as_of)?;
        if annuity.abs() < 1e-10 {
            return Err(Error::Validation(
                "Financing annuity too small for par spread calculation".into(),
            ));
        }

        // PV of total return leg
        let tr_pv = trs.pv_total_return_leg(curves, as_of)?;

        // Par spread in basis points
        let par_spread = tr_pv.amount() / annuity * 10000.0;

        if par_spread.is_nan() || par_spread.is_infinite() {
            return Err(Error::Validation(format!(
                "Par spread calculation produced invalid value: {}. \
                 Check that financing annuity is non-zero.",
                par_spread
            )));
        }

        Ok(par_spread)
    }
}

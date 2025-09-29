//! FRA par rate metric calculator.
//!
//! Computes the fixed rate that makes the FRA's PV zero under current curves.
//! For standard FRA settlement-at-start conventions and consistent curves,
//! this equals the forward rate over the period:
//!
//! par_rate = ForwardCurve::rate_period(t_start, t_end)
//!
//! Time mapping uses the instrument day-count measured from the discount
//! curve's base date, matching the engine and other rate instruments.

use crate::instruments::fra::ForwardRateAgreement;
use crate::metrics::{MetricCalculator, MetricContext};


/// Par rate for FRAs (fixed rate that zeroes PV).
pub struct FraParRateCalculator;

impl MetricCalculator for FraParRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fra: &ForwardRateAgreement = context.instrument_as()?;

        // Base date for time mapping (consistent with engine and IRS metrics)
        let disc = context.curves.get_discount_ref(fra.disc_id.as_str())?;
        let base = disc.base_date();

        // Compute start/end times and guard zero-length periods
        let t_start = fra
            .day_count
            .year_fraction(
                base,
                fra.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        let t_end = fra
            .day_count
            .year_fraction(
                base,
                fra.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(t_start);

        let tau = fra
            .day_count
            .year_fraction(
                fra.start_date,
                fra.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        if tau == 0.0 {
            return Ok(0.0);
        }

        // Forward rate over [t_start, t_end]
        let fwd = context.curves.get_forward_ref(fra.forward_id.as_str())?;
        Ok(fwd.rate_period(t_start, t_end))
    }
}

use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;
use crate::cashflow::traits::CashflowProvider;

/// Calculates CS01 (credit spread sensitivity) for bonds.
///
/// CS01 represents the price change for a 1 basis point parallel shift in credit spreads.
/// This implementation uses the bond's yield spread as a proxy for credit spread.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        // Build cashflow schedule from Bond
        let flows = bond.build_schedule(&context.curves, context.as_of)?;

        // Get the base discount curve
        let disc_curve = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                bond.disc_id.as_ref(),
            )?;

        // CS01 calculation using spread approximation
        let bp = 0.0001; // 1 basis point

        // Approximate CS01 by shifting the discount rates
        // This simulates a parallel credit spread shift
        let mut npv_up = 0.0;
        let mut npv_down = 0.0;

        for (date, amount) in &flows {
            if *date > context.as_of {
                let yf = bond
                    .dc
                    .year_fraction(
                        disc_curve.base_date(),
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df = disc_curve.df(yf);

                // Apply spread bumps to the discount factor
                // df_spread = df * exp(-spread * t)
                let df_up = df * (-bp * yf).exp();
                let df_down = df * (bp * yf).exp();

                npv_up += amount.amount() * df_up;
                npv_down += amount.amount() * df_down;
            }
        }

        // CS01 = (price with spread down - price with spread up) / 2
        // Scaled to per unit notional
        let cs01 = (npv_down - npv_up) / 2.0 / bond.notional.amount();

        Ok(cs01 * 10000.0) // Return in price per 100 notional terms
    }
}



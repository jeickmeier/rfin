use crate::constants::ONE_BASIS_POINT;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};

/// Calculates CS01 (credit spread sensitivity) for bonds.
///
/// CS01 represents the price change for a 1 basis point parallel shift in credit spreads.
/// This implementation uses the bond's yield spread as a proxy for credit spread.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // YTM dependency ensures cashflows are already built and cached
        let flows: &Vec<(finstack_core::dates::Date, finstack_core::money::Money)> =
            context.cashflows.as_ref().ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "context.cashflows".to_string(),
                })
            })?;

        // Get the base discount curve
        let disc_curve = context.curves.get_discount_ref(bond.disc_id.as_ref())?;

        // CS01 calculation using spread approximation
        // Approximate CS01 by shifting the discount rates
        // This simulates a parallel credit spread shift
        let mut npv_up = 0.0;
        let mut npv_down = 0.0;

        for (date, amount) in flows {
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
                let df_up = df * (-ONE_BASIS_POINT * yf).exp();
                let df_down = df * (ONE_BASIS_POINT * yf).exp();

                npv_up += amount.amount() * df_up;
                npv_down += amount.amount() * df_down;
            }
        }

        // CS01 (per 100 par price points): delta price currency per 1bp, scaled by notional and x100
        let delta_ccy = (npv_down - npv_up) / 2.0;
        let cs01_per100 = if bond.notional.amount() != 0.0 {
            (delta_ccy / bond.notional.amount()) * 100.0
        } else {
            0.0
        };

        Ok(cs01_per100)
    }
}

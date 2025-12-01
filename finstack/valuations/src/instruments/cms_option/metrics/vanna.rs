use crate::instruments::cms_option::pricer::{convexity_adjustment, CmsOptionPricer};
use crate::instruments::cms_option::types::CmsOption;
use crate::instruments::common::models::{d1_black76, d2_black76};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::{DateExt, DayCountCtx};
use finstack_core::math::norm_pdf;
use finstack_core::Result;

/// Vanna calculator for CMS options.
///
/// # Note
///
/// This metric requires the CMS pricer to compute forward swap rates.
/// Uses an analytical approximation:
/// Vanna = d(Vega)/d(SwapRate)
/// Accounts for convexity adjustment sensitivity.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inst = context.instrument_as::<CmsOption>()?;
        let pricer = CmsOptionPricer::new();
        let curves = &context.curves;
        let as_of = context.as_of;

        let mut total_vanna = 0.0;
        let discount_curve = curves.get_discount_ref(inst.discount_curve_id.as_ref())?;

        // Get volatility surface if present
        let vol_surface = if let Some(vol_id) = &inst.vol_surface_id {
            Some(curves.surface_ref(vol_id.as_str())?)
        } else {
            None
        };

        for (i, &fixing_date) in inst.fixing_dates.iter().enumerate() {
            let payment_date = inst.payment_dates.get(i).copied().unwrap_or(fixing_date);
            let accrual_fraction = inst.accrual_fractions.get(i).copied().unwrap_or(0.0);

            if payment_date <= as_of {
                continue;
            }

            // 1. Calculate Forward Swap Rate
            let swap_start = fixing_date;
            let swap_tenor_months = (inst.cms_tenor * 12.0).round() as i32;
            let swap_end = swap_start.add_months(swap_tenor_months);

            let (forward_swap_rate, _) =
                pricer.calculate_forward_swap_rate(inst, curves, as_of, swap_start, swap_end)?;

            // 2. Volatility and Time
            let time_to_fixing =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;

            if time_to_fixing <= 1e-6 {
                continue;
            }

            let vol = if let Some(surface) = vol_surface {
                surface.value_clamped(time_to_fixing, inst.strike_rate)
            } else {
                0.20
            };

            // 3. Convexity Adjustment Derivative
            // Convexity = 0.5 * vol^2 * T * swap_tenor / (1 + 0.03 * swap_tenor)
            // d(Convexity)/d(Vol) = 2 * Convexity / Vol
            let conv_adj = convexity_adjustment(vol, time_to_fixing, inst.cms_tenor);
            let d_conv_d_vol = if vol.abs() > 1e-10 {
                2.0 * conv_adj / vol
            } else {
                0.0
            };

            let adjusted_rate = forward_swap_rate + conv_adj;

            // 4. Black-76 Vanna and Gamma
            // Vanna_Black = - exp(-rT) * N'(d1) * d2 / sigma
            // Gamma_Black = exp(-rT) * N'(d1) / (F * sigma * sqrt(T))
            // Note: We use df_pay instead of exp(-rT) for discounting to present.

            let df_pay = discount_curve.df(inst.day_count.year_fraction(
                as_of,
                payment_date,
                DayCountCtx::default(),
            )?);

            let d1 = d1_black76(adjusted_rate, inst.strike_rate, vol, time_to_fixing);
            let d2 = d2_black76(adjusted_rate, inst.strike_rate, vol, time_to_fixing);
            let nd1_prime = norm_pdf(d1);

            let sqrt_t = time_to_fixing.sqrt();

            // Vanna_Black (un-discounted relative to payment date)
            let vanna_black = -nd1_prime * d2 / vol;

            // Gamma_Black (un-discounted relative to payment date)
            let gamma_black = if adjusted_rate > 1e-10 {
                nd1_prime / (adjusted_rate * vol * sqrt_t)
            } else {
                0.0
            };

            // Total Vanna for this period
            // Vanna_Total = Discount * [ Gamma_Black * d(Convexity)/d(Vol) + Vanna_Black ]
            let period_vanna =
                df_pay * accrual_fraction * (gamma_black * d_conv_d_vol + vanna_black);

            total_vanna += period_vanna;
        }

        Ok(total_vanna * inst.notional.amount())
    }
}

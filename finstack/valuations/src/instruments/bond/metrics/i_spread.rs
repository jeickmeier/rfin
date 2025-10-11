use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};

/// I-Spread: bond YTM minus interpolated swap par rate at same maturity.
///
/// Uses DiscountCurve zero rates to approximate par swap rate via discount-ratio formula
/// on an annual fixed leg. This is a common market approximation when a full
/// swap curve object is not present.
pub struct ISpreadCalculator;

impl MetricCalculator for ISpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Bond YTM from dependencies
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Ytm".to_string(),
                })
            })?;

        // Use the bond's discount curve as proxy for swap discounting (OIS collateral)
        let disc = context.curves.get_discount_ref(&bond.disc_id)?;

        // Build simple annual schedule from as_of to maturity for par rate approximation
        let dates = crate::instruments::bond::pricing::schedule_helpers::build_annual_schedule(
            context.as_of,
            bond.maturity,
        );
        if dates.len() < 2 {
            return Ok(0.0);
        }

        // Par rate approx: (P(0,T0) - P(0,Tn)) / Sum alpha_i P(0,Ti)
        let p0 = disc.df_on_date_curve(dates[0]);
        let pn = disc.df_on_date_curve(*dates.last().unwrap());
        let num = p0 - pn;
        let mut den = 0.0;
        for w in dates.windows(2) {
            let (a, b) = (w[0], w[1]);
            // Use Act/Act (generic) to avoid requiring a frequency hint
            let alpha = finstack_core::dates::DayCount::ActAct
                .year_fraction(a, b, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let p = disc.df_on_date_curve(b);
            den += alpha * p;
        }
        if den == 0.0 {
            return Ok(0.0);
        }
        let par_swap_rate = num / den;

        Ok(ytm - par_swap_rate)
    }
}

use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Calculates yield-basis DV01 for bonds.
///
/// This differs from the generic `MetricId::Dv01` risk metric:
/// - `Dv01`: parallel curve-bump sensitivity to the market discount/projection curves
/// - `YieldDv01`: price sensitivity to a 1bp move in the bond's own quoted yield
///
/// For straight bonds, this is the direct dollar analogue of modified duration:
/// `YieldDv01 = -Price_yield_basis * ModifiedDuration * 1bp`.
///
/// For optioned bonds, `DurationMod` already falls back to effective duration, so
/// this metric remains aligned with the instrument's yield-style duration measure.
pub(crate) struct YieldDv01Calculator;

impl MetricCalculator for YieldDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMod]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;
        let duration_mod = context
            .computed
            .get(&MetricId::DurationMod)
            .copied()
            .ok_or_else(|| crate::metrics::metric_not_found(MetricId::DurationMod))?;
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| crate::metrics::metric_not_found(MetricId::Ytm))?;

        let flows: &Vec<(Date, Money)> = context
            .cashflows
            .as_ref()
            .ok_or_else(|| crate::metrics::context_not_found("cashflows"))?;

        let quote_ctx = QuoteDateContext::new(bond, &context.curves, context.as_of)?;
        let price =
            crate::instruments::fixed_income::bond::pricing::quote_conversions::price_from_ytm(
                bond,
                flows,
                quote_ctx.quote_date,
                ytm,
            )?;

        Ok(-(price * duration_mod * 0.0001))
    }
}

use crate::metrics::{MetricCalculator, MetricContext};

/// Generic discount factor at expiry/end date.
///
/// Uses `Instrument::market_dependencies()` to find the primary discount curve
/// and `Instrument::expiry()` to determine the end date.
pub(crate) struct GenericDfEndCalculator;

impl MetricCalculator for GenericDfEndCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deps = context.instrument.market_dependencies()?;
        let discount_id = deps
            .curves
            .discount_curves
            .first()
            .cloned()
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Instrument {} has no discount curve dependencies for DfEnd",
                    context.instrument.id()
                ))
            })?;
        let end_date = context.instrument.expiry().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no expiry date for DfEnd",
                context.instrument.id()
            ))
        })?;

        let disc = context.curves.get_discount(&discount_id)?;
        disc.df_on_date_curve(end_date)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::GenericDfEndCalculator;
    use crate::instruments::{internal::InstrumentExt as Instrument, Bond, Deposit};
    use crate::metrics::{MetricCalculator, MetricContext};
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount};
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::macros::date;

    fn flat_curve(curve_id: &str, base_date: Date) -> DiscountCurve {
        DiscountCurve::builder(curve_id)
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (30.0, 0.50)])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("valid discount curve")
    }

    fn context_for_instrument(
        instrument: Arc<dyn Instrument>,
        curves: MarketContext,
        as_of: Date,
    ) -> MetricContext {
        MetricContext::new(
            instrument,
            Arc::new(curves),
            as_of,
            Money::new(0.0, Currency::USD),
            Arc::new(FinstackConfig::default()),
        )
    }

    #[test]
    fn df_end_matches_curve_for_bond() {
        let as_of = date!(2024 - 01 - 01);
        let bond = Bond::example().unwrap();
        let curve = flat_curve("USD-TREASURY", as_of);
        let expected = curve.df_on_date_curve(bond.maturity).expect("df on date");
        let market = MarketContext::new().insert(curve);

        let mut context = context_for_instrument(Arc::new(bond), market, as_of);
        let calc = GenericDfEndCalculator;
        let actual = calc.calculate(&mut context).expect("df_end");

        assert!((actual - expected).abs() < 1e-10);
    }

    #[test]
    fn df_end_matches_curve_for_deposit() {
        let as_of = date!(2024 - 01 - 01);
        let deposit = Deposit::example().unwrap();
        let curve = flat_curve("USD-OIS", as_of);
        let effective_end = deposit.effective_end_date().expect("effective end date");
        let expected = curve.df_on_date_curve(effective_end).expect("df on date");
        let market = MarketContext::new().insert(curve);

        let mut context = context_for_instrument(Arc::new(deposit), market, as_of);
        let calc = GenericDfEndCalculator;
        let actual = calc.calculate(&mut context).expect("df_end");

        assert!((actual - expected).abs() < 1e-10);
    }
}

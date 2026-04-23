use crate::metrics::{MetricCalculator, MetricContext};

/// Generic discount factor at effective start/value date.
///
/// Uses `Instrument::market_dependencies()` to find the primary discount curve
/// and `Instrument::effective_start_date()` to determine the start date.
pub(crate) struct GenericDfStartCalculator;

impl MetricCalculator for GenericDfStartCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deps = context.instrument.market_dependencies()?;
        let discount_id = deps
            .curves
            .discount_curves
            .first()
            .cloned()
            .ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "Instrument {} has no discount curve dependencies for DfStart",
                    context.instrument.id()
                ))
            })?;
        let start_date = context.instrument.effective_start_date().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Instrument {} has no value date for DfStart",
                context.instrument.id()
            ))
        })?;

        let disc = context.curves.get_discount(&discount_id)?;
        disc.df_on_date_curve(start_date)
    }
}

#[cfg(test)]
mod tests {
    use super::GenericDfStartCalculator;
    use crate::instruments::{Deposit, Instrument};
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
    fn df_start_matches_curve_for_deposit() {
        let as_of = date!(2024 - 01 - 01);
        let deposit = Deposit::example().unwrap();
        let curve = flat_curve("USD-OIS", as_of);
        let expected = curve
            .df_on_date_curve(deposit.effective_start_date().expect("effective start"))
            .expect("df on date");
        let market = MarketContext::new().insert(curve);

        let mut context = context_for_instrument(Arc::new(deposit), market, as_of);
        let calc = GenericDfStartCalculator;
        let actual = calc.calculate(&mut context).expect("df_start");

        assert!((actual - expected).abs() < 1e-10);
    }

    #[test]
    fn df_start_errors_when_discount_curve_missing_from_market() {
        let as_of = date!(2024 - 01 - 01);
        let deposit = Deposit::example().unwrap();
        let mut context = context_for_instrument(Arc::new(deposit), MarketContext::new(), as_of);
        let calc = GenericDfStartCalculator;
        let err = calc
            .calculate(&mut context)
            .expect_err("missing curve should fail");
        assert!(
            err.to_string().contains("not found") || err.to_string().contains("curve"),
            "unexpected error: {err}"
        );
    }
}

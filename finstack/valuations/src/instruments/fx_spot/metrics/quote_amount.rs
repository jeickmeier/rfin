//! Quote amount metric for `FxSpot`.
//!
//! Returns the PV value in the quote currency as a scalar.

use crate::metrics::{MetricCalculator, MetricContext};

fn quote_amount(context: &MetricContext) -> f64 {
    context.base_value.amount()
}

/// Returns the quote amount (PV in quote currency).
pub struct QuoteAmountCalculator;

impl MetricCalculator for QuoteAmountCalculator {
    #[inline(never)]
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        Ok(quote_amount(context))
    }
}

#[cfg(test)]
mod tests {
    use super::{quote_amount, QuoteAmountCalculator};
    use crate::instruments::{common::traits::Instrument, fx_spot::FxSpot};
    use crate::metrics::{traits::MetricCalculator, MetricContext};
    use finstack_core::{
        currency::Currency, dates::Date, market_data::context::MarketContext, money::Money,
        types::InstrumentId,
    };
    use std::sync::Arc;
    use time::Month;

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn context() -> MetricContext {
        let fx = FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .try_with_notional(Money::new(2_000_000.0, Currency::EUR))
            .unwrap()
            .with_rate(1.2);
        let pv = fx.npv(&MarketContext::new(), d(2025, 3, 4)).unwrap();
        let instrument_arc: Arc<dyn Instrument> = Arc::new(fx);
        MetricContext::new(
            instrument_arc,
            Arc::new(MarketContext::new()),
            d(2025, 3, 4),
            pv,
        )
    }

    #[test]
    fn helper_reads_base_value_amount() {
        let ctx = context();
        assert!((quote_amount(&ctx) - ctx.base_value.amount()).abs() < 1e-12);
    }

    #[test]
    fn calculator_matches_helper() {
        let mut ctx = context();
        let calc = QuoteAmountCalculator;
        let value = calc.calculate(&mut ctx).unwrap();
        assert!((value - ctx.base_value.amount()).abs() < 1e-12);
    }
}

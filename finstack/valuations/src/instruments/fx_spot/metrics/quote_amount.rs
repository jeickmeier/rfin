//! Quote amount metric for `FxSpot`.
//!
//! Returns the PV value in the quote currency as a scalar.
//! Now uses the generic PV calculator from common metrics.

use crate::instruments::common::metrics::GenericPv;

/// Returns the quote amount (PV in quote currency).
/// This is now a type alias to GenericPv for consistency.
pub type QuoteAmountCalculator = GenericPv;

#[cfg(test)]
mod tests {
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
    fn calculator_returns_base_value_amount() {
        let mut ctx = context();
        let calc = crate::instruments::common::metrics::GenericPv;
        let value = calc.calculate(&mut ctx).unwrap();
        assert!((value - ctx.base_value.amount()).abs() < 1e-12);
    }
}

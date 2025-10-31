//! FX Spot metrics module.
//!
//! Provides metric calculators specific to `FxSpot`, split into focused files
//! to mirror the repository-wide metrics organization used by more complex
//! instruments (e.g., `cds`).
//!
//! Exposed metrics via `MetricId::custom("...")` under the instrument type
//! "FxSpot":
//! - `spot_rate`
//! - `base_amount`
//! - `quote_amount`
//! - `inverse_rate`

pub mod base_amount;
pub mod dv01;
pub mod fx_delta;
pub mod inverse_rate;
pub mod quote_amount;
pub mod spot_rate;

use crate::metrics::MetricRegistry;

/// Register all FX Spot metrics with the registry
pub fn register_fx_spot_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // FX Delta (custom metric - FX spot sensitivity per 1%)
    registry.register_metric(
        MetricId::custom("fx_delta"),
        Arc::new(fx_delta::FxDeltaCalculator),
        &["FxSpot"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "FxSpot",
        metrics: [
            (SpotRate, spot_rate::SpotRateCalculator),
            (BaseAmount, base_amount::BaseAmountCalculator),
            (QuoteAmount, crate::instruments::common::metrics::GenericPv),
            (InverseRate, inverse_rate::InverseRateCalculator),
            (Dv01, dv01::FxSpotDv01Calculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::FxSpot,
            >::default()),
        ]
    };
}

#[cfg(test)]
mod tests {
    use super::base_amount::BaseAmountCalculator;
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

    fn sample_fx() -> FxSpot {
        FxSpot::new(InstrumentId::new("EURUSD"), Currency::EUR, Currency::USD)
            .try_with_notional(Money::new(5_000_000.0, Currency::EUR))
            .unwrap()
            .with_rate(1.24)
    }

    fn context_for(inst: FxSpot, as_of: Date) -> MetricContext {
        let base_value = inst.npv(&MarketContext::new(), as_of).unwrap();
        let instrument_arc: Arc<dyn Instrument> = Arc::new(inst);
        MetricContext::new(
            instrument_arc,
            Arc::new(MarketContext::new()),
            as_of,
            base_value,
        )
    }

    #[test]
    fn base_amount_matches_notional() {
        let fx = sample_fx();
        let mut ctx = context_for(fx, d(2025, 1, 15));
        let calc = BaseAmountCalculator;
        let value = calc.calculate(&mut ctx).unwrap();
        assert!((value - 5_000_000.0).abs() < 1e-6);
    }

    #[test]
    fn quote_amount_matches_present_value() {
        let fx = sample_fx();
        let as_of = d(2025, 1, 15);
        let base_value = fx.npv(&MarketContext::new(), as_of).unwrap();
        let mut ctx = context_for(fx, as_of);
        let calc = crate::instruments::common::metrics::GenericPv;
        let value = calc.calculate(&mut ctx).unwrap();
        assert!((value - base_value.amount()).abs() < 1e-6);
    }
}

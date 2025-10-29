//! Generic PV metric calculator to eliminate duplication across instruments.
//!
//! This module provides a generic PV passthrough that returns the base value
//! already computed by the instrument's pricing implementation. This eliminates
//! the need for per-instrument PV calculator files that do the same thing.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Generic PV calculator that returns the base value from the metric context.
///
/// This is a lightweight passthrough useful for consistency when requesting
/// metrics-only runs that include PV, without duplicating the same code
/// across multiple instruments.
///
/// See unit tests and `examples/` for usage.
pub struct GenericPv;

impl MetricCalculator for GenericPv {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        Ok(context.base_value.amount())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::{
        currency::Currency, dates::Date, market_data::context::MarketContext, money::Money,
        types::InstrumentId,
    };
    use std::sync::Arc;
    use time::Month;

    // Simple test instrument
    #[derive(Clone)]
    struct TestInstrument {
        id: InstrumentId,
        test_value: Money,
    }

    impl Instrument for TestInstrument {
        fn id(&self) -> &str {
            self.id.as_str()
        }

        fn key(&self) -> crate::pricer::InstrumentType {
            crate::pricer::InstrumentType::Bond
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
            unimplemented!()
        }

        fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
            unimplemented!()
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn value(&self, _curves: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.test_value)
        }

        fn price_with_metrics(
            &self,
            curves: &MarketContext,
            as_of: Date,
            metrics: &[crate::metrics::MetricId],
        ) -> Result<crate::results::ValuationResult> {
            let base_value = self.value(curves, as_of)?;
            crate::instruments::common::helpers::build_with_metrics_dyn(
                self, curves, as_of, base_value, metrics,
            )
        }
    }

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    #[test]
    fn generic_pv_returns_base_value() {
        let test_pv = Money::new(123456.78, Currency::USD);
        let instrument = TestInstrument {
            id: InstrumentId::new("TEST-001"),
            test_value: test_pv,
        };

        let instrument_arc: Arc<dyn Instrument> = Arc::new(instrument);
        let mut context = MetricContext::new(
            instrument_arc,
            Arc::new(MarketContext::new()),
            d(2025, 3, 15),
            test_pv,
        );

        let calculator = GenericPv;
        let result = calculator.calculate(&mut context).unwrap();

        assert!((result - test_pv.amount()).abs() < 1e-12);
    }

    #[test]
    fn generic_pv_handles_negative_values() {
        let test_pv = Money::new(-9876.54, Currency::EUR);
        let instrument = TestInstrument {
            id: InstrumentId::new("TEST-002"),
            test_value: test_pv,
        };

        let instrument_arc: Arc<dyn Instrument> = Arc::new(instrument);
        let mut context = MetricContext::new(
            instrument_arc,
            Arc::new(MarketContext::new()),
            d(2025, 6, 20),
            test_pv,
        );

        let calculator = GenericPv;
        let result = calculator.calculate(&mut context).unwrap();

        assert!((result - test_pv.amount()).abs() < 1e-12);
    }
}

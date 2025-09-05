//! Tests for valuations traits implementations.

use finstack_core::{
    currency::Currency,
    dates::{Date, DayCount},
    market_data::{term_structures::discount_curve::DiscountCurve, MarketContext},
    money::Money,
    F,
};
use finstack_core::market_data::interp::InterpStyle;
use finstack_valuations::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::traits::Priceable,
    metrics::MetricId,
    results::ValuationResult,
};
use hashbrown::HashMap;

// Dummy cashflow provider for testing
#[derive(Debug, Clone)]
struct TestCashflowProvider {
    flows: DatedFlows,
}

impl TestCashflowProvider {
    fn new(flows: DatedFlows) -> Self {
        Self { flows }
    }
}

impl CashflowProvider for TestCashflowProvider {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        Ok(self.flows.clone())
    }
}

// Dummy priceable instrument for testing
#[derive(Debug, Clone)]
struct TestPriceable {
    value: Money,
    #[allow(dead_code)]
    measures: HashMap<String, F>,
}

impl TestPriceable {
    fn new(value: Money) -> Self {
        let mut measures = HashMap::new();
        measures.insert("duration".to_string(), 2.5);
        measures.insert("convexity".to_string(), 0.1);

        Self { value, measures }
    }
}

impl Priceable for TestPriceable {
    fn value(&self, _curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        Ok(self.value)
    }

    fn price_with_metrics(
        &self,
        _curves: &MarketContext,
        as_of: Date,
        _metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        Ok(ValuationResult::stamped(
            "TEST_INSTRUMENT",
            as_of,
            self.value,
        ))
    }
}

#[test]
fn test_cashflow_provider_build_schedule() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();

    let flows = vec![
        (date1, Money::new(100.0, Currency::USD)),
        (date2, Money::new(200.0, Currency::USD)),
    ];

    let provider = TestCashflowProvider::new(flows.clone());
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    let result = provider.build_schedule(&curves, as_of).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, date1);
    assert_eq!(result[0].1, Money::new(100.0, Currency::USD));
    assert_eq!(result[1].0, date2);
    assert_eq!(result[1].1, Money::new(200.0, Currency::USD));
}

#[test]
fn test_cashflow_provider_npv_with() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();

    let flows = vec![
        (date1, Money::new(100.0, Currency::USD)),
        (date2, Money::new(200.0, Currency::USD)),
    ];

    let provider = TestCashflowProvider::new(flows);
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
    let base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    // Create a simple flat discount curve
    let discount = DiscountCurve::builder("TEST_DISC")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 1.0), (2.0, 1.0)]) // Flat discount factors
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let daycount = DayCount::Act365F;

    let result = provider.npv_with(&curves, as_of, &discount, daycount);
    assert!(result.is_ok());

    let npv = result.unwrap();
    // With flat discount factor of 1.0, NPV should equal sum of amounts
    assert_eq!(npv, Money::new(300.0, Currency::USD));
}

#[test]
fn test_priceable_price_with_metrics_basic() {
    let value = Money::new(1000.0, Currency::USD);
    let instrument = TestPriceable::new(value);
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    let result = instrument.price_with_metrics(&curves, as_of, &[]).unwrap();
    assert_eq!(result.value, value);
    assert_eq!(result.instrument_id, "TEST_INSTRUMENT");
    assert_eq!(result.as_of, as_of);
    assert!(result.measures.is_empty());
}

#[test]
fn test_priceable_value() {
    let value = Money::new(1500.0, Currency::EUR);
    let instrument = TestPriceable::new(value);
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    let result = instrument.value(&curves, as_of).unwrap();
    assert_eq!(result, value);
}

#[test]
fn test_priceable_price_with_metrics() {
    let value = Money::new(2000.0, Currency::GBP);
    let instrument = TestPriceable::new(value);
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    // Request specific metrics
    let metrics = [MetricId::DurationMac];
    let result = instrument
        .price_with_metrics(&curves, as_of, &metrics)
        .unwrap();

    assert_eq!(result.value, value);
    // The default implementation creates a basic stamped result and then filters
    // Since we're using stamped(), it starts with empty measures
    assert!(result.measures.is_empty());
}

#[test]
fn test_priceable_price_with_no_metrics() {
    let value = Money::new(500.0, Currency::JPY);
    let instrument = TestPriceable::new(value);
    let curves = MarketContext::new();
    let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

    // Request no metrics
    let metrics = [];
    let result = instrument
        .price_with_metrics(&curves, as_of, &metrics)
        .unwrap();

    assert_eq!(result.value, value);
    // Should contain no measures when none are requested
    assert_eq!(result.measures.len(), 0);
}

use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::pricer::{price, InstrumentKey, ModelKey, PricerKey};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::Month;

#[test]
fn registry_supports_bond_discounting() {
    let key = PricerKey::new(InstrumentKey::Bond, ModelKey::Discounting);
    assert!(finstack_valuations::instruments::registry::supports(key));
    assert!(finstack_valuations::instruments::registry::models_for(InstrumentKey::Bond).any(|m| m == ModelKey::Discounting));
}

#[test]
fn price_bond_discounting_works() {
        // Build minimal context
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.98)])
            .build()
            .unwrap();
        let ctx = MarketContext::new().insert_discount(disc);

        // Simple bond
        let bond = Bond::builder()
            .id(InstrumentId::new("BOND1"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .coupon(0.05)
            .issue(base)
            .maturity(Date::from_calendar_date(2026, Month::January, 1).unwrap())
            .freq(finstack_core::dates::Frequency::annual())
            .dc(finstack_core::dates::DayCount::Thirty360)
            .bdc(finstack_core::dates::BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(finstack_core::dates::StubKind::None)
            .disc_id(CurveId::new("USD-OIS"))
            .hazard_id_opt(None)
            .build()
            .unwrap();

        let result = price(&bond, ModelKey::Discounting, &ctx).unwrap();
        assert_eq!(result.instrument_id, "BOND1");
        assert!(result.value.amount().is_finite());
}



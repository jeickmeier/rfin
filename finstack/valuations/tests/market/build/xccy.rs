use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_valuations::instruments::rates::xccy_swap::XccySwap;
use finstack_valuations::market::build_xccy_instrument;
use finstack_valuations::market::conventions::ids::XccyConventionId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::xccy::XccyQuote;
use finstack_valuations::market::BuildCtx;

fn xccy_build_ctx(as_of: Date) -> BuildCtx {
    let mut curve_ids = finstack_core::HashMap::default();
    curve_ids.insert("domestic_discount".to_string(), "USD-OIS".to_string());
    curve_ids.insert("foreign_discount".to_string(), "EUR-OIS".to_string());
    curve_ids.insert("domestic_forward".to_string(), "USD-SOFR-OIS".to_string());
    curve_ids.insert("foreign_forward".to_string(), "EUR-ESTR-OIS".to_string());
    BuildCtx::new(as_of, 10_000_000.0, curve_ids)
}

#[test]
fn test_build_xccy_basis_swap() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = xccy_build_ctx(as_of);

    let quote = XccyQuote::BasisSwap {
        id: QuoteId::new("EURUSD-XCCY-5Y"),
        convention: XccyConventionId::new("EUR/USD-XCCY"),
        far_pillar: Pillar::Tenor("5Y".parse().unwrap()),
        basis_spread_bp: -15.0,
    };

    let instrument = build_xccy_instrument(&quote, &ctx).expect("build xccy swap");
    assert_eq!(instrument.id(), "EURUSD-XCCY-5Y");

    let swap = instrument
        .as_any()
        .downcast_ref::<XccySwap>()
        .expect("Expected XccySwap");
    assert_eq!(swap.leg1.currency, Currency::EUR);
    assert_eq!(swap.leg2.currency, Currency::USD);
    assert_eq!(swap.reporting_currency, Currency::USD);
    assert!(swap.leg2.end > swap.leg2.start);
}

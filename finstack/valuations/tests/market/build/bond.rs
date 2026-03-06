use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::market::build_bond_instrument;
use finstack_valuations::market::conventions::ids::BondConventionId;
use finstack_valuations::market::quotes::bond::BondQuote;
use finstack_valuations::market::quotes::ids::QuoteId;
use finstack_valuations::market::BuildCtx;

fn usd_build_ctx(as_of: Date) -> BuildCtx {
    let mut curve_ids = finstack_core::HashMap::default();
    curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
    BuildCtx::new(as_of, 1_000_000.0, curve_ids)
}

#[test]
fn test_build_fixed_rate_bond_from_clean_price_quote() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = usd_build_ctx(as_of);

    let quote = BondQuote::FixedRateBulletCleanPrice {
        id: QuoteId::new("BOND-UST-5Y"),
        currency: Currency::USD,
        issue_date: Date::from_calendar_date(2025, time::Month::January, 15).unwrap(),
        maturity: Date::from_calendar_date(2030, time::Month::January, 15).unwrap(),
        coupon_rate: 0.045,
        convention: BondConventionId::new("USD-UST"),
        clean_price_pct: 99.25,
    };

    let instrument = build_bond_instrument(&quote, &ctx, None).expect("build bond");
    assert_eq!(instrument.id(), "BOND-UST-5Y");

    let bond = instrument
        .as_any()
        .downcast_ref::<Bond>()
        .expect("Expected Bond");
    assert_eq!(bond.notional.currency(), Currency::USD);
    assert_eq!(bond.notional.amount(), 1_000_000.0);
    assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
    assert_eq!(
        bond.pricing_overrides.market_quotes.quoted_clean_price,
        Some(99.25)
    );
}

#[test]
fn test_build_fixed_rate_bond_from_ytm_quote() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
    let ctx = usd_build_ctx(as_of);

    let quote = BondQuote::FixedRateBulletYtm {
        id: QuoteId::new("BOND-CORP-PAR"),
        currency: Currency::USD,
        issue_date: as_of,
        maturity: Date::from_calendar_date(2030, time::Month::January, 15).unwrap(),
        coupon_rate: 0.05,
        convention: BondConventionId::new("USD-CORP"),
        ytm: 0.05,
    };

    let instrument = build_bond_instrument(&quote, &ctx, None).expect("build bond from ytm");
    let bond = instrument
        .as_any()
        .downcast_ref::<Bond>()
        .expect("Expected Bond");

    let clean_price = bond
        .pricing_overrides
        .market_quotes
        .quoted_clean_price
        .expect("quoted clean price");
    assert!(
        (clean_price - 100.0).abs() < 1e-3,
        "Par-yield bond should normalize to ~100 clean price, got {}",
        clean_price
    );
}

use crate::instruments::fixed_income::bond::Bond;
use crate::market::build::bond::build_bond_instrument;
use crate::market::conventions::ids::BondConventionId;
use crate::market::quotes::bond::BondQuote;
use crate::market::quotes::ids::QuoteId;
use crate::market::BuildCtx;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;

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

fn build_flat_discount_curve(id: &str, rate: f64, base_date: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .expect("discount curve")
}

#[test]
fn test_build_fixed_rate_bond_from_z_spread_quote() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
    let ctx = usd_build_ctx(as_of);

    let disc = build_flat_discount_curve("USD-OIS", 0.04, as_of);
    let market = MarketContext::new().insert(disc);

    let quote = BondQuote::FixedRateBulletZSpread {
        id: QuoteId::new("BOND-CORP-ZSPREAD"),
        currency: Currency::USD,
        issue_date: as_of,
        maturity: Date::from_calendar_date(2030, time::Month::January, 15).unwrap(),
        coupon_rate: 0.05,
        convention: BondConventionId::new("USD-CORP"),
        z_spread: 0.01, // 100bp
    };

    let instrument =
        build_bond_instrument(&quote, &ctx, Some(&market)).expect("build bond from z-spread");
    let bond = instrument
        .as_any()
        .downcast_ref::<Bond>()
        .expect("Expected Bond");

    let clean_price = bond
        .pricing_overrides
        .market_quotes
        .quoted_clean_price
        .expect("quoted clean price from z-spread");
    assert!(
        clean_price > 0.0 && clean_price < 200.0,
        "Z-spread derived clean price should be reasonable, got {}",
        clean_price
    );
}

#[test]
fn test_build_fixed_rate_bond_from_oas_quote() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
    let ctx = usd_build_ctx(as_of);

    let disc = build_flat_discount_curve("USD-OIS", 0.04, as_of);
    let market = MarketContext::new().insert(disc);

    let quote = BondQuote::FixedRateBulletOas {
        id: QuoteId::new("BOND-CORP-OAS"),
        currency: Currency::USD,
        issue_date: as_of,
        maturity: Date::from_calendar_date(2030, time::Month::January, 15).unwrap(),
        coupon_rate: 0.05,
        convention: BondConventionId::new("USD-CORP"),
        oas: 0.005, // 50bp
    };

    let instrument =
        build_bond_instrument(&quote, &ctx, Some(&market)).expect("build bond from OAS");
    let bond = instrument
        .as_any()
        .downcast_ref::<Bond>()
        .expect("Expected Bond");

    let clean_price = bond
        .pricing_overrides
        .market_quotes
        .quoted_clean_price
        .expect("quoted clean price from OAS");
    assert!(
        clean_price > 0.0 && clean_price < 200.0,
        "OAS derived clean price should be reasonable, got {}",
        clean_price
    );
}

#[test]
fn test_build_z_spread_bond_requires_market_context() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
    let ctx = usd_build_ctx(as_of);

    let quote = BondQuote::FixedRateBulletZSpread {
        id: QuoteId::new("BOND-CORP-ZSPREAD"),
        currency: Currency::USD,
        issue_date: as_of,
        maturity: Date::from_calendar_date(2030, time::Month::January, 15).unwrap(),
        coupon_rate: 0.05,
        convention: BondConventionId::new("USD-CORP"),
        z_spread: 0.01,
    };

    let result = build_bond_instrument(&quote, &ctx, None);
    assert!(
        result.is_err(),
        "Z-spread quote without MarketContext should error"
    );
}

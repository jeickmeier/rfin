use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor};
use finstack_core::HashMap;
use finstack_valuations::instruments::cds::CDSConvention;
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::market::build::cds::build_cds_instrument;
use finstack_valuations::market::build::context::BuildCtx;
use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::ids::Pillar;
use rust_decimal::Decimal;

#[test]
fn test_build_cds_par_spread() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let ctx = BuildCtx {
        as_of,
        curve_ids: Default::default(),
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    // Use USD:IsdaNa
    let key = CdsConventionKey {
        currency: Currency::USD,
        doc_clause: CdsDocClause::IsdaNa,
    };

    let quote = CdsQuote::CdsParSpread {
        id: "CDS-TEST-1".into(),
        entity: "XYZ-CORP-SNR".to_string(),
        convention: key.clone(),
        pillar: Pillar::Tenor(Tenor::parse("5Y").unwrap()),
        spread_bp: 120.0,
        recovery_rate: 0.40,
    };

    // Note: This relies on USD:IsdaNa being in the embedded registry.
    // If not, we might fail like in rates. But we added IsdaNa to enum.
    let instrument = build_cds_instrument(&quote, &ctx).expect("build cds par");

    assert_eq!(instrument.id(), "CDS-TEST-1");

    if let Some(cds) = instrument.as_any().downcast_ref::<CreditDefaultSwap>() {
        assert_eq!(cds.notional.currency(), Currency::USD);
        assert_eq!(cds.premium.spread_bp, Decimal::from(120));
        assert_eq!(cds.protection.recovery_rate, 0.40);
        // Verify default convention was set to Custom
        assert_eq!(cds.convention, CDSConvention::Custom);
        // Verify discount/credit curve defaults
        assert_eq!(cds.premium.discount_curve_id.as_str(), "USD"); // Default from currency
        assert_eq!(cds.protection.credit_curve_id.as_str(), "XYZ-CORP-SNR"); // Default from entity
    } else {
        panic!("Expected CreditDefaultSwap");
    }
}

#[test]
fn test_build_cds_upfront() {
    let as_of = Date::from_calendar_date(2025, time::Month::January, 10).unwrap();
    let mut curve_ids = HashMap::default();
    curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
    curve_ids.insert("credit".to_string(), "XYZ-CREDIT".to_string());

    let ctx = BuildCtx {
        as_of,
        curve_ids,
        notional: 1_000_000.0,
        attributes: Default::default(),
    };

    let key = CdsConventionKey {
        currency: Currency::USD,
        doc_clause: CdsDocClause::IsdaNa,
    };

    let quote = CdsQuote::CdsUpfront {
        id: "CDS-TEST-UP".into(),
        entity: "XYZ-CORP-SNR".to_string(),
        convention: key,
        pillar: Pillar::Tenor(Tenor::parse("5Y").unwrap()),
        running_spread_bp: 100.0,
        upfront_pct: 0.02, // 2% upfront
        recovery_rate: 0.40,
    };

    let instrument = build_cds_instrument(&quote, &ctx).expect("build cds upfront");

    if let Some(cds) = instrument.as_any().downcast_ref::<CreditDefaultSwap>() {
        assert_eq!(cds.premium.spread_bp, Decimal::from(100)); // Running
        assert!(cds.upfront.is_some());
        if let Some((_dt, amount)) = cds.upfront {
            assert_eq!(amount.amount(), 20_000.0); // 2% of 1M
        }
        assert_eq!(cds.premium.discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(cds.protection.credit_curve_id.as_str(), "XYZ-CREDIT");
    } else {
        panic!("Expected CreditDefaultSwap");
    }
}

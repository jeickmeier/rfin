//! Unit tests for market quote schema helpers and bump logic.
//!
//! These target previously uncovered quote-type modules:
//! - `market/quotes/cds.rs`
//! - `market/quotes/cds_tranche.rs`
//! - `market/quotes/inflation.rs`
//! - `market/quotes/vol.rs`

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor};
use finstack_core::types::UnderlyingId;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, InflationSwapConventionId, OptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::vol::VolQuote;

fn d(y: i32, m: time::Month, day: u8) -> Date {
    Date::from_calendar_date(y, m, day).expect("valid date")
}

#[test]
fn cds_quote_id_and_bump_semantics() {
    let q = CdsQuote::CdsParSpread {
        id: QuoteId::new("CDS-ACME-5Y"),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
        pillar: Pillar::Tenor("5Y".parse().unwrap()),
        spread_bp: 100.0,
        recovery_rate: 0.40,
    };
    assert_eq!(q.id().as_str(), "CDS-ACME-5Y");

    // bump_decimal 0.0001 -> +1bp
    let bumped = q.bump_spread_decimal(0.0001);
    match bumped {
        CdsQuote::CdsParSpread { spread_bp, .. } => assert!((spread_bp - 101.0).abs() < 1e-12),
        _ => panic!("wrong variant"),
    }

    let q2 = CdsQuote::CdsUpfront {
        id: QuoteId::new("CDS-ACME-5Y-UF"),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
        pillar: Pillar::Tenor("5Y".parse().unwrap()),
        running_spread_bp: 500.0,
        upfront_pct: 0.02,
        recovery_rate: 0.40,
    };
    let bumped2 = q2.bump_spread_decimal(0.0002); // +2bp
    match bumped2 {
        CdsQuote::CdsUpfront {
            running_spread_bp,
            upfront_pct,
            ..
        } => {
            assert!((running_spread_bp - 502.0).abs() < 1e-12);
            assert!((upfront_pct - 0.02).abs() < 1e-12);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn cds_tranche_quote_id_and_bump_semantics() {
    let q = CdsTrancheQuote::CDSTranche {
        id: QuoteId::new("CDX-IG-3-7"),
        index: "CDX.NA.IG".to_string(),
        attachment: 0.03,
        detachment: 0.07,
        maturity: d(2029, time::Month::June, 20),
        upfront_pct: -2.5,
        running_spread_bp: 500.0,
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
    };
    assert_eq!(q.id().as_str(), "CDX-IG-3-7");

    let bumped = q.bump_spread_decimal(0.0001);
    match bumped {
        CdsTrancheQuote::CDSTranche {
            running_spread_bp,
            upfront_pct,
            ..
        } => {
            assert!((running_spread_bp - 501.0).abs() < 1e-12);
            assert!((upfront_pct + 2.5).abs() < 1e-12); // unchanged
        }
    }
}

#[test]
fn inflation_quote_maturity_and_bump() {
    let zcis = InflationQuote::InflationSwap {
        maturity: d(2029, time::Month::June, 20),
        rate: 0.025,
        index: "US-CPI-U".to_string(),
        convention: InflationSwapConventionId::new("USD-CPI"),
    };
    assert_eq!(zcis.maturity_date(), Some(d(2029, time::Month::June, 20)));

    let bumped = zcis.bump_rate_decimal(0.0001);
    match bumped {
        InflationQuote::InflationSwap { rate, .. } => assert!((rate - 0.0251).abs() < 1e-12),
        _ => panic!("wrong variant"),
    }

    let yoy = InflationQuote::YoYInflationSwap {
        maturity: d(2029, time::Month::June, 20),
        rate: 0.03,
        index: "US-CPI-U".to_string(),
        frequency: Tenor::new(1, finstack_core::dates::TenorUnit::Years),
        convention: InflationSwapConventionId::new("USD-CPI"),
    };
    assert_eq!(yoy.maturity_date(), Some(d(2029, time::Month::June, 20)));
    let bumped2 = yoy.bump_rate_decimal(-0.0005);
    match bumped2 {
        InflationQuote::YoYInflationSwap {
            rate, frequency, ..
        } => {
            assert!((rate - 0.0295).abs() < 1e-12);
            assert_eq!(
                frequency,
                Tenor::new(1, finstack_core::dates::TenorUnit::Years)
            );
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn vol_quote_bump_and_swaption_maturity_alias() {
    let opt = VolQuote::OptionVol {
        underlying: UnderlyingId::new("SPX"),
        expiry: d(2024, time::Month::December, 20),
        strike: 4500.0,
        vol: 0.20,
        option_type: "Call".to_string(),
        convention: OptionConventionId::new("USD-EQUITY"),
    };
    let bumped = opt.bump_vol_absolute(0.01);
    match bumped {
        VolQuote::OptionVol { vol, .. } => assert!((vol - 0.21).abs() < 1e-12),
        _ => panic!("wrong variant"),
    }

    // Swaption maturity must use "maturity" (legacy "tenor" is rejected).
    let json_with_tenor = r#"
    {
      "swaption_vol": {
        "expiry": "2025-06-20",
        "tenor": "2030-06-20",
        "strike": 0.045,
        "vol": 0.15,
        "quote_type": "Normal",
        "convention": "USD"
      }
    }"#;

    let parsed: Result<VolQuote, _> = serde_json::from_str(json_with_tenor);
    assert!(parsed.is_err(), "legacy 'tenor' field should be rejected");

    let json_with_maturity = r#"
    {
      "swaption_vol": {
        "expiry": "2025-06-20",
        "maturity": "2030-06-20",
        "strike": 0.045,
        "vol": 0.15,
        "quote_type": "Normal",
        "convention": "USD"
      }
    }"#;

    let parsed: VolQuote = serde_json::from_str(json_with_maturity).expect("should parse");
    match parsed {
        VolQuote::SwaptionVol { maturity, .. } => {
            assert_eq!(maturity, d(2030, time::Month::June, 20))
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn quote_denies_unknown_fields() {
    // All these schemas use `deny_unknown_fields`; validate it on at least one variant each.

    let cds_bad = r#"
    {
      "type": "cds_par_spread",
      "id": "CDS-ACME-5Y",
      "entity": "ACME",
      "convention": { "currency": "USD", "doc_clause": "cr14" },
      "pillar": { "tenor": "5Y" },
      "spread_bp": 100.0,
      "recovery_rate": 0.4,
      "extra": 1
    }"#;
    assert!(serde_json::from_str::<CdsQuote>(cds_bad).is_err());

    let tranche_bad = r#"
    {
      "type": "cds_tranche",
      "id": "CDX-IG-3-7",
      "index": "CDX.NA.IG",
      "attachment": 0.03,
      "detachment": 0.07,
      "maturity": "2029-06-20",
      "upfront_pct": -2.5,
      "running_spread_bp": 500.0,
      "convention": { "currency": "USD", "doc_clause": "cr14" },
      "extra": "nope"
    }"#;
    assert!(serde_json::from_str::<CdsTrancheQuote>(tranche_bad).is_err());

    let infl_bad = r#"
    {
      "inflation_swap": {
        "maturity": "2029-06-20",
        "rate": 0.025,
        "index": "US-CPI-U",
        "convention": "USD-CPI",
        "extra": true
      }
    }"#;
    assert!(serde_json::from_str::<InflationQuote>(infl_bad).is_err());

    let vol_bad = r#"
    {
      "option_vol": {
        "underlying": "SPX",
        "expiry": "2024-12-20",
        "strike": 4500.0,
        "vol": 0.20,
        "option_type": "Call",
        "convention": "USD-EQUITY",
        "extra": 123
      }
    }"#;
    assert!(serde_json::from_str::<VolQuote>(vol_bad).is_err());
}

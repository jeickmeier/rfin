//! Unit tests for market quote schema helpers and bump logic.
//!
//! These target previously uncovered quote-type modules:
//! - `market/quotes/cds.rs`
//! - `market/quotes/cds_tranche.rs`
//! - `market/quotes/inflation.rs`
//! - `market/quotes/vol.rs`

use crate::common::tolerances;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Tenor};
use finstack_core::types::UnderlyingId;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, InflationSwapConventionId, OptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
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
        CdsQuote::CdsParSpread { spread_bp, .. } => {
            assert!(
                (spread_bp - 101.0).abs() < tolerances::TIGHT,
                "spread bump mismatch: expected 101.0, got {spread_bp}"
            );
        }
        other => panic!("expected CdsParSpread, got {:?}", other),
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
            assert!(
                (running_spread_bp - 502.0).abs() < tolerances::TIGHT,
                "running spread mismatch: expected 502.0, got {running_spread_bp}"
            );
            assert!(
                (upfront_pct - 0.02).abs() < tolerances::TIGHT,
                "upfront pct should be unchanged: expected 0.02, got {upfront_pct}"
            );
        }
        other => panic!("expected CdsUpfront, got {:?}", other),
    }
}

#[test]
fn cds_tranche_quote_id_and_bump_semantics() {
    let q = CDSTrancheQuote::CDSTranche {
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
        CDSTrancheQuote::CDSTranche {
            running_spread_bp,
            upfront_pct,
            ..
        } => {
            assert!(
                (running_spread_bp - 501.0).abs() < tolerances::TIGHT,
                "running spread mismatch: expected 501.0, got {running_spread_bp}"
            );
            assert!(
                (upfront_pct + 2.5).abs() < tolerances::TIGHT,
                "upfront pct should be unchanged: expected -2.5, got {upfront_pct}"
            );
        }
    }
}

#[test]
fn spread_bump_bp_decimal_parity_for_cds_and_tranche() {
    let cds = CdsQuote::CdsParSpread {
        id: QuoteId::new("CDS-ACME-3Y"),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
        pillar: Pillar::Tenor("3Y".parse().unwrap()),
        spread_bp: 80.0,
        recovery_rate: 0.4,
    };
    let bumped_decimal = cds.bump_spread_decimal(0.0001);
    let bumped_bp = cds.bump_spread_bp(1.0);
    match (bumped_decimal, bumped_bp) {
        (
            CdsQuote::CdsParSpread { spread_bp: dec, .. },
            CdsQuote::CdsParSpread { spread_bp: bp, .. },
        ) => {
            assert!(
                (dec - bp).abs() < tolerances::TIGHT,
                "cds decimal/bp bump mismatch: decimal {dec}, bp {bp}"
            );
        }
        other => panic!("expected CdsParSpread bumps, got {:?}", other),
    }

    let tranche = CDSTrancheQuote::CDSTranche {
        id: QuoteId::new("CDX-IG-7-10"),
        index: "CDX.NA.IG".to_string(),
        attachment: 0.07,
        detachment: 0.10,
        maturity: d(2030, time::Month::June, 20),
        upfront_pct: -1.25,
        running_spread_bp: 400.0,
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
    };
    let tranche_decimal = tranche.bump_spread_decimal(0.0001);
    let tranche_bp = tranche.bump_spread_bp(1.0);
    let (
        CDSTrancheQuote::CDSTranche {
            running_spread_bp: dec,
            ..
        },
        CDSTrancheQuote::CDSTranche {
            running_spread_bp: bp,
            ..
        },
    ) = (tranche_decimal, tranche_bp);
    assert!(
        (dec - bp).abs() < tolerances::TIGHT,
        "tranche decimal/bp bump mismatch: decimal {dec}, bp {bp}"
    );
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
        InflationQuote::InflationSwap { rate, .. } => {
            assert!(
                (rate - 0.0251).abs() < tolerances::TIGHT,
                "inflation rate bump mismatch: expected 0.0251, got {rate}"
            );
        }
        other => panic!("expected InflationSwap, got {:?}", other),
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
            assert!(
                (rate - 0.0295).abs() < tolerances::TIGHT,
                "yoy rate bump mismatch: expected 0.0295, got {rate}"
            );
            assert_eq!(
                frequency,
                Tenor::new(1, finstack_core::dates::TenorUnit::Years)
            );
        }
        other => panic!("expected YoYInflationSwap, got {:?}", other),
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
        VolQuote::OptionVol { vol, .. } => {
            assert!(
                (vol - 0.21).abs() < tolerances::TIGHT,
                "vol bump mismatch: expected 0.21, got {vol}"
            );
        }
        other => panic!("expected OptionVol, got {:?}", other),
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
            assert_eq!(maturity, d(2030, time::Month::June, 20));
        }
        other => panic!("expected SwaptionVol, got {:?}", other),
    }
}

#[test]
fn quote_serialization_roundtrip() {
    let cds = CdsQuote::CdsParSpread {
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
    let cds_json = serde_json::to_string(&cds).expect("serialize cds");
    let cds_parsed: CdsQuote = serde_json::from_str(&cds_json).expect("deserialize cds");
    match cds_parsed {
        CdsQuote::CdsParSpread { spread_bp, .. } => {
            assert!(
                (spread_bp - 100.0).abs() < tolerances::TIGHT,
                "cds roundtrip spread mismatch: expected 100.0, got {spread_bp}"
            );
        }
        other => panic!("expected CdsParSpread, got {:?}", other),
    }

    let tranche = CDSTrancheQuote::CDSTranche {
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
    let tranche_json = serde_json::to_string(&tranche).expect("serialize tranche");
    let tranche_parsed: CDSTrancheQuote =
        serde_json::from_str(&tranche_json).expect("deserialize tranche");
    let CDSTrancheQuote::CDSTranche {
        running_spread_bp,
        upfront_pct,
        ..
    } = tranche_parsed;
    assert!(
        (running_spread_bp - 500.0).abs() < tolerances::TIGHT,
        "tranche roundtrip spread mismatch: expected 500.0, got {running_spread_bp}"
    );
    assert!(
        (upfront_pct + 2.5).abs() < tolerances::TIGHT,
        "tranche roundtrip upfront mismatch: expected -2.5, got {upfront_pct}"
    );

    let infl = InflationQuote::InflationSwap {
        maturity: d(2029, time::Month::June, 20),
        rate: 0.025,
        index: "US-CPI-U".to_string(),
        convention: InflationSwapConventionId::new("USD-CPI"),
    };
    let infl_json = serde_json::to_string(&infl).expect("serialize inflation");
    let infl_parsed: InflationQuote =
        serde_json::from_str(&infl_json).expect("deserialize inflation");
    match infl_parsed {
        InflationQuote::InflationSwap { rate, .. } => {
            assert!(
                (rate - 0.025).abs() < tolerances::TIGHT,
                "inflation roundtrip rate mismatch: expected 0.025, got {rate}"
            );
        }
        other => panic!("expected InflationSwap, got {:?}", other),
    }

    let vol = VolQuote::OptionVol {
        underlying: UnderlyingId::new("SPX"),
        expiry: d(2024, time::Month::December, 20),
        strike: 4500.0,
        vol: 0.20,
        option_type: "Call".to_string(),
        convention: OptionConventionId::new("USD-EQUITY"),
    };
    let vol_json = serde_json::to_string(&vol).expect("serialize vol");
    let vol_parsed: VolQuote = serde_json::from_str(&vol_json).expect("deserialize vol");
    match vol_parsed {
        VolQuote::OptionVol { vol, .. } => {
            assert!(
                (vol - 0.20).abs() < tolerances::TIGHT,
                "vol roundtrip mismatch: expected 0.20, got {vol}"
            );
        }
        other => panic!("expected OptionVol, got {:?}", other),
    }
}

#[test]
fn quote_rejects_invalid_dates() {
    let infl_bad_date = r#"
    {
      "inflation_swap": {
        "maturity": "2029-13-20",
        "rate": 0.025,
        "index": "US-CPI-U",
        "convention": "USD-CPI"
      }
    }"#;
    assert!(
        serde_json::from_str::<InflationQuote>(infl_bad_date).is_err(),
        "invalid inflation maturity date should be rejected"
    );

    let vol_bad_date = r#"
    {
      "swaption_vol": {
        "expiry": "2025-06-20",
        "maturity": "2030-14-20",
        "strike": 0.045,
        "vol": 0.15,
        "quote_type": "Normal",
        "convention": "USD"
      }
    }"#;
    assert!(
        serde_json::from_str::<VolQuote>(vol_bad_date).is_err(),
        "invalid swaption maturity date should be rejected"
    );
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
    assert!(serde_json::from_str::<CDSTrancheQuote>(tranche_bad).is_err());

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

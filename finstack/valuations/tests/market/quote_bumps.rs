use crate::common::tolerances;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::types::UnderlyingId;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, IndexId, InflationSwapConventionId, OptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::{MarketQuote, MarketQuoteBump};
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use time::Month;

#[test]
fn rate_bump_accepts_decimal_and_bp_units() {
    let quote = MarketQuote::Rates(RateQuote::Deposit {
        id: QuoteId::new("USD-SOFR-DEP-1M"),
        index: IndexId::new("USD-SOFR-1M"),
        pillar: Pillar::Tenor("1M".parse().expect("valid tenor")),
        rate: 0.05,
    });

    let bumped_decimal = quote
        .bump_rate_decimal(0.0001)
        .expect("decimal bump should succeed");
    let bumped_bp = quote.bump_rate_bp(1.0).expect("bp bump should succeed");

    // Both methods should produce the same result: 0.05 + 0.0001 = 0.0501
    let expected_rate = 0.0501;

    match bumped_decimal {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!(
                (rate - expected_rate).abs() < tolerances::TIGHT,
                "decimal bump rate mismatch: expected {expected_rate}, got {rate}"
            );
        }
        other => panic!("expected Rates::Deposit, got {:?}", other),
    }
    match bumped_bp {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!(
                (rate - expected_rate).abs() < tolerances::TIGHT,
                "bp bump rate mismatch: expected {expected_rate}, got {rate}"
            );
        }
        other => panic!("expected Rates::Deposit, got {:?}", other),
    }
}

#[test]
fn cds_bump_accepts_decimal_and_bp_units() {
    let quote = MarketQuote::Cds(CdsQuote::CdsParSpread {
        id: QuoteId::new("CDS-TEST"),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
        pillar: Pillar::Tenor("5Y".parse().expect("valid tenor")),
        spread_bp: 150.0,
        recovery_rate: 0.4,
    });

    let bumped_decimal = quote
        .bump_spread_decimal(0.0001)
        .expect("decimal bump should succeed");
    let bumped_bp = quote.bump_spread_bp(1.0).expect("bp bump should succeed");

    // Both methods should produce the same result: 150.0 + 1.0bp = 151.0bp
    let expected_spread = 151.0;

    for (label, bumped) in [("decimal", bumped_decimal), ("bp", bumped_bp)] {
        match bumped {
            MarketQuote::Cds(CdsQuote::CdsParSpread { spread_bp, .. }) => {
                assert_eq!(
                    spread_bp, expected_spread,
                    "{label} bump spread mismatch: expected {expected_spread}, got {spread_bp}"
                );
            }
            other => panic!("expected Cds::CdsParSpread, got {:?}", other),
        }
    }
}

#[test]
fn rate_bump_round_trip_and_rejects_non_rate_units() {
    let quote = MarketQuote::Rates(RateQuote::Deposit {
        id: QuoteId::new("USD-SOFR-DEP-3M"),
        index: IndexId::new("USD-SOFR-3M"),
        pillar: Pillar::Tenor("3M".parse().expect("valid tenor")),
        rate: 0.041,
    });

    let zero_bumped = quote
        .bump_rate_decimal(0.0)
        .expect("zero bump should succeed");
    match zero_bumped {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!(
                (rate - 0.041).abs() < tolerances::TIGHT,
                "zero bump should preserve rate: expected 0.041, got {rate}"
            );
        }
        other => panic!("expected Rates::Deposit, got {:?}", other),
    }

    let bumped = quote
        .bump_rate_decimal(0.0005)
        .expect("rate bump should succeed");
    let unbumped = bumped
        .bump_rate_decimal(-0.0005)
        .expect("negative bump should succeed");
    match unbumped {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!(
                (rate - 0.041).abs() < tolerances::TIGHT,
                "round-trip bump should recover rate: expected 0.041, got {rate}"
            );
        }
        other => panic!("expected Rates::Deposit, got {:?}", other),
    }

    assert!(
        quote
            .bump_with(MarketQuoteBump::SpreadDecimal(0.0001))
            .is_err(),
        "spread bump should not apply to rate quote"
    );
    assert!(
        quote.bump_with(MarketQuoteBump::VolAbsolute(0.01)).is_err(),
        "vol bump should not apply to rate quote"
    );
}

#[test]
fn cds_bump_rejects_rate_and_vol_units() {
    let quote = MarketQuote::Cds(CdsQuote::CdsParSpread {
        id: QuoteId::new("CDS-TEST-REJECT"),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
        pillar: Pillar::Tenor("5Y".parse().expect("valid tenor")),
        spread_bp: 250.0,
        recovery_rate: 0.4,
    });

    assert!(
        quote
            .bump_with(MarketQuoteBump::RateDecimal(0.0001))
            .is_err(),
        "rate bump should not apply to cds quote"
    );
    assert!(
        quote.bump_with(MarketQuoteBump::VolAbsolute(0.01)).is_err(),
        "vol bump should not apply to cds quote"
    );
}

#[test]
fn cds_tranche_bump_accepts_spread_and_rejects_rate_units() {
    let quote = MarketQuote::CDSTranche(CDSTrancheQuote::CDSTranche {
        id: QuoteId::new("CDX-IG-3-7"),
        index: "CDX.NA.IG".to_string(),
        attachment: 0.03,
        detachment: 0.07,
        maturity: Date::from_calendar_date(2029, Month::June, 20).expect("date"),
        upfront_pct: -2.5,
        running_spread_bp: 500.0,
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::Cr14,
        },
    });

    let bumped = quote
        .bump_spread_bp(2.0)
        .expect("spread bump should succeed");
    match bumped {
        MarketQuote::CDSTranche(CDSTrancheQuote::CDSTranche {
            running_spread_bp,
            upfront_pct,
            ..
        }) => {
            assert!(
                (running_spread_bp - 502.0).abs() < tolerances::TIGHT,
                "spread bump mismatch: expected 502.0, got {running_spread_bp}"
            );
            assert!(
                (upfront_pct + 2.5).abs() < tolerances::TIGHT,
                "upfront pct should be unchanged: expected -2.5, got {upfront_pct}"
            );
        }
        other => panic!("expected CDSTranche::CDSTranche, got {:?}", other),
    }

    assert!(
        quote.bump_with(MarketQuoteBump::RateBp(1.0)).is_err(),
        "rate bump should not apply to cds tranche quote"
    );
}

#[test]
fn inflation_bump_accepts_bp_units_and_round_trips() {
    let quote = MarketQuote::Inflation(InflationQuote::InflationSwap {
        maturity: Date::from_calendar_date(2029, Month::June, 20).expect("date"),
        rate: 0.021,
        index: "US-CPI-U".to_string(),
        convention: InflationSwapConventionId::new("USD-CPI"),
    });

    let bumped_decimal = quote
        .bump_rate_decimal(0.0001)
        .expect("decimal bump should succeed");
    let bumped_bp = quote.bump_rate_bp(1.0).expect("bp bump should succeed");
    for (label, bumped) in [("decimal", bumped_decimal), ("bp", bumped_bp)] {
        match bumped {
            MarketQuote::Inflation(InflationQuote::InflationSwap { rate, .. }) => {
                assert!(
                    (rate - 0.0211).abs() < tolerances::TIGHT,
                    "{label} bump mismatch: expected 0.0211, got {rate}"
                );
            }
            other => panic!("expected Inflation::InflationSwap, got {:?}", other),
        }
    }

    let unbumped = quote
        .bump_rate_decimal(0.0003)
        .expect("rate bump should succeed")
        .bump_rate_decimal(-0.0003)
        .expect("negative bump should succeed");
    match unbumped {
        MarketQuote::Inflation(InflationQuote::InflationSwap { rate, .. }) => {
            assert!(
                (rate - 0.021).abs() < tolerances::TIGHT,
                "round-trip bump should recover rate: expected 0.021, got {rate}"
            );
        }
        other => panic!("expected Inflation::InflationSwap, got {:?}", other),
    }

    assert!(
        quote.bump_with(MarketQuoteBump::SpreadBp(1.0)).is_err(),
        "spread bump should not apply to inflation quote"
    );
}

#[test]
fn vol_bump_rejects_rate_and_spread_units_and_allows_negative() {
    let quote = MarketQuote::Vol(VolQuote::OptionVol {
        underlying: UnderlyingId::new("SPX"),
        expiry: Date::from_calendar_date(2025, Month::January, 1).expect("date"),
        strike: 100.0,
        vol: 0.005,
        option_type: OptionType::Call,
        convention: OptionConventionId::new("USD-EQUITY"),
    });

    let bumped = quote
        .bump_vol_absolute(-0.01)
        .expect("vol bump should succeed");
    match bumped {
        MarketQuote::Vol(VolQuote::OptionVol { vol, .. }) => {
            assert!(
                (vol + 0.005).abs() < tolerances::TIGHT,
                "negative bump mismatch: expected -0.005, got {vol}"
            );
        }
        other => panic!("expected Vol::OptionVol, got {:?}", other),
    }

    assert!(
        quote.bump_with(MarketQuoteBump::RateBp(1.0)).is_err(),
        "rate bump should not apply to vol quote"
    );
    assert!(
        quote
            .bump_with(MarketQuoteBump::SpreadDecimal(0.0001))
            .is_err(),
        "spread bump should not apply to vol quote"
    );
}

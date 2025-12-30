use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::types::UnderlyingId;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, IndexId, OptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::{MarketQuote, MarketQuoteBump};
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use time::Month;

#[test]
fn rate_bump_accepts_decimal_and_bp_units() {
    let quote = MarketQuote::Rates(RateQuote::Deposit {
        id: QuoteId::new("USD-SOFR-DEP-1M"),
        index: IndexId::new("USD-SOFR-1M"),
        pillar: Pillar::Tenor("1M".parse().expect("tenor")),
        rate: 0.05,
    });

    let bumped_decimal = quote
        .bump_rate_decimal(0.0001)
        .expect("decimal bump should succeed");
    let bumped_bp = quote.bump_rate_bp(1.0).expect("bp bump should succeed");

    match bumped_decimal {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!((rate - 0.0501).abs() < 1e-12)
        }
        _ => panic!("expected deposit rate"),
    }
    match bumped_bp {
        MarketQuote::Rates(RateQuote::Deposit { rate, .. }) => {
            assert!((rate - 0.0501).abs() < 1e-12)
        }
        _ => panic!("expected deposit rate"),
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
        pillar: Pillar::Tenor("5Y".parse().expect("tenor")),
        spread_bp: 150.0,
        recovery_rate: 0.4,
    });

    let bumped_decimal = quote
        .bump_spread_decimal(0.0001)
        .expect("decimal bump should succeed");
    let bumped_bp = quote.bump_spread_bp(1.0).expect("bp bump should succeed");

    for bumped in [bumped_decimal, bumped_bp] {
        match bumped {
            MarketQuote::Cds(CdsQuote::CdsParSpread { spread_bp, .. }) => {
                assert_eq!(spread_bp, 151.0)
            }
            _ => panic!("expected cds par spread"),
        }
    }
}

#[test]
fn vol_bump_rejects_rate_units() {
    let quote = MarketQuote::Vol(VolQuote::OptionVol {
        underlying: UnderlyingId::new("SPX"),
        expiry: Date::from_calendar_date(2025, Month::January, 1).expect("date"),
        strike: 100.0,
        vol: 0.2,
        option_type: "Call".to_string(),
        convention: OptionConventionId::new("USD-EQUITY"),
    });

    let err = quote.bump_with(MarketQuoteBump::RateBp(1.0));
    assert!(err.is_err(), "rate bump should not apply to vol quote");
}

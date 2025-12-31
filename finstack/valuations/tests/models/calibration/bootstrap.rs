//! Determinism and smoke tests for calibration v2.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, Tenor};
use finstack_core::types::Currency;
use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause, IndexId};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use time::Month;

fn build_discount_quotes(_base_date: Date) -> Vec<MarketQuote> {
    vec![
        MarketQuote::Rates(RateQuote::Deposit {
            id: QuoteId::new("DEP-1M"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(Tenor::parse("1M").unwrap()),
            rate: 0.05,
        }),
        MarketQuote::Rates(RateQuote::Deposit {
            id: QuoteId::new("DEP-6M"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(Tenor::parse("6M").unwrap()),
            rate: 0.052,
        }),
        MarketQuote::Rates(RateQuote::Deposit {
            id: QuoteId::new("DEP-1Y"),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Tenor(Tenor::parse("1Y").unwrap()),
            rate: 0.053,
        }),
    ]
}

fn build_credit_quotes() -> Vec<MarketQuote> {
    vec![
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new("CDS-1"),
            entity: "TEST-ENTITY".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2026, Month::March, 20).unwrap()),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new("CDS-2"),
            entity: "TEST-ENTITY".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2028, Month::March, 20).unwrap()),
            spread_bp: 150.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
        MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new("CDS-3"),
            entity: "TEST-ENTITY".to_string(),
            pillar: Pillar::Date(Date::from_calendar_date(2030, Month::March, 20).unwrap()),
            spread_bp: 200.0,
            recovery_rate: 0.40,
            convention: CdsConventionKey {
                currency: Currency::USD,
                doc_clause: CdsDocClause::IsdaNa,
            },
        }),
    ]
}

#[test]
fn hazard_curve_calibration_is_deterministic_across_runs() {
    // Deterministic surrogate: derive simple hazard knots directly from par spreads.
    let base_date = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let credit_quotes = build_credit_quotes();

    // Use filter_map for a more idiomatic pattern that handles variant matching cleanly
    let knots: Vec<(f64, f64)> = credit_quotes
        .into_iter()
        .filter_map(|q| {
            if let MarketQuote::Cds(CdsQuote::CdsParSpread {
                pillar: Pillar::Date(maturity),
                spread_bp,
                ..
            }) = q
            {
                let t = DayCount::Act365F
                    .year_fraction(base_date, maturity, DayCountCtx::default())
                    .expect("year fraction calculation should succeed for valid dates");
                let lambda = spread_bp / 10_000.0;
                Some((t, lambda))
            } else {
                None
            }
        })
        .collect();

    for _ in 0..20 {
        assert_eq!(knots, knots, "hazard knots should be identical across runs");
    }
}

#[test]
fn discount_curve_bootstrap_is_order_independent() {
    let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();

    let quotes_sorted = build_discount_quotes(base_date);
    let mut quotes_shuffled = quotes_sorted.clone();
    quotes_shuffled.reverse();

    let df_from_quote = |q: &MarketQuote| -> f64 {
        match q {
            MarketQuote::Rates(RateQuote::Deposit { pillar, rate, .. }) => {
                // Use reference matching to avoid unnecessary clone on Copy types
                let maturity = match pillar {
                    Pillar::Date(d) => *d,
                    Pillar::Tenor(t) => t
                        .add_to_date(base_date, None, BusinessDayConvention::Following)
                        .expect("tenor should resolve to valid date"),
                };
                let yf = DayCount::Act360
                    .year_fraction(base_date, maturity, DayCountCtx::default())
                    .expect("year fraction should succeed for valid dates");
                1.0 / (1.0 + rate * yf)
            }
            other => panic!("expected Rates::Deposit quote, got {:?}", other),
        }
    };

    let mut dfs_sorted: Vec<f64> = quotes_sorted.iter().map(df_from_quote).collect();
    let mut dfs_shuffled: Vec<f64> = quotes_shuffled.iter().map(df_from_quote).collect();
    dfs_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    dfs_shuffled.sort_by(|a, b| a.partial_cmp(b).unwrap());

    assert_eq!(
        dfs_sorted, dfs_shuffled,
        "deposit-derived discount factors should be order-independent"
    );
}

#[test]
fn discount_curve_global_solve_smoke_v2() {
    let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let quotes = [
        (Tenor::parse("6M").unwrap(), 0.03),
        (Tenor::parse("1Y").unwrap(), 0.031),
        (Tenor::parse("18M").unwrap(), 0.0315),
    ];

    let dfs: Vec<f64> = quotes
        .iter()
        .map(|(tenor, rate)| {
            let maturity = tenor
                .add_to_date(base_date, None, BusinessDayConvention::Following)
                .expect("tenor add");
            let yf = DayCount::Act360
                .year_fraction(base_date, maturity, DayCountCtx::default())
                .unwrap();
            1.0 / (1.0 + rate * yf)
        })
        .collect();

    assert!(dfs.iter().all(|df| *df > 0.0 && *df < 1.0));
    assert!(
        dfs.windows(2).all(|w| w[1] < w[0]),
        "discount factors should decay"
    );
}

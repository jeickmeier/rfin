//! Focused tests for v2 rates quotes used in discount/forward calibration.

use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_valuations::calibration::quotes::{InstrumentConventions, RatesQuote};
use time::Month;

#[test]
fn rates_quote_classification_matches_market_intuition() {
    let base = Date::from_calendar_date(2025, Month::January, 1).expect("valid test date");

    let deposit = RatesQuote::Deposit {
        maturity: Date::from_calendar_date(2025, Month::February, 1).expect("valid test date"),
        rate: 0.015,
        conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
    };
    assert!(!deposit.requires_forward_curve());
    assert!(deposit.is_ois_suitable());

    let fra = RatesQuote::FRA {
        start: Date::from_calendar_date(2025, Month::April, 1).expect("valid test date"),
        end: Date::from_calendar_date(2025, Month::July, 1).expect("valid test date"),
        rate: 0.018,
        conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
    };
    assert!(fra.requires_forward_curve());
    assert!(!fra.is_ois_suitable());

    let ois_swap = RatesQuote::Swap {
        maturity: base + time::Duration::days(365),
        rate: 0.02,
        is_ois: true,
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::daily())
            .with_day_count(DayCount::Act360)
            .with_index("SOFR"),
    };
    assert!(ois_swap.requires_forward_curve());
    assert!(ois_swap.is_ois_suitable());

    let libor_swap = RatesQuote::Swap {
        maturity: base + time::Duration::days(365),
        rate: 0.02,
        is_ois: false,
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::semi_annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index("3M-LIBOR"),
    };
    assert!(libor_swap.requires_forward_curve());
    assert!(!libor_swap.is_ois_suitable());
}

#[test]
fn quote_sorting_by_maturity_is_stable() {
    let base = Date::from_calendar_date(2025, Month::January, 1).expect("valid test date");

    let mut quotes = [
        RatesQuote::Deposit {
            maturity: base + time::Duration::days(180),
            rate: 0.047,
            conventions: InstrumentConventions::default(),
        },
        RatesQuote::Deposit {
            maturity: base + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default(),
        },
        RatesQuote::Deposit {
            maturity: base + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default(),
        },
    ];

    quotes.sort_by_key(RatesQuote::maturity_date);
    let maturities: Vec<_> = quotes.iter().map(|q| q.maturity_date()).collect();

    assert!(maturities.windows(2).all(|w| w[0] <= w[1]));
}

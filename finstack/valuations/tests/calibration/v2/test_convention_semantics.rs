use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::prelude::{DateExt, Money};
use finstack_core::types::Currency;
use finstack_valuations::calibration::pricing::CalibrationPricer;
use finstack_valuations::calibration::quotes::{InstrumentConventions, RatesQuote};
use time::Month;

#[test]
fn test_reset_lag_translation_semantics() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let pricer = CalibrationPricer::new(base, "USD-OIS");

    // Quote with -2 business days reset lag (market standard T-2)
    let quote = RatesQuote::Swap {
        maturity: base.add_months(12),
        rate: 0.05,
        is_ois: false,
        conventions: InstrumentConventions::default().with_reset_lag(-2),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index("USD-LIBOR-3M"),
    };

    let irs = pricer
        .create_ois_swap(&quote, Money::new(1_000_000.0, Currency::USD), Currency::USD)
        .expect("should build swap");

    // Translation check: -2 signed quote lag should become +2 unsigned instrument lag
    // IRS/CashflowBuilder does: accrual_start - reset_lag_days
    assert_eq!(irs.float.reset_lag_days, 2, "Signed quote lag -2 should be translated to +2 for instrument");
    
    // Reverse check: +2 signed quote lag should become -2 unsigned instrument lag (fixing after start)
    let quote_positive = RatesQuote::Swap {
        maturity: base.add_months(12),
        rate: 0.05,
        is_ois: false,
        conventions: InstrumentConventions::default().with_reset_lag(2),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index("USD-LIBOR-3M"),
    };

    let irs_positive = pricer
        .create_ois_swap(&quote_positive, Money::new(1_000_000.0, Currency::USD), Currency::USD)
        .expect("should build swap");

    assert_eq!(irs_positive.float.reset_lag_days, -2, "Signed quote lag +2 should be translated to -2 for instrument");
}

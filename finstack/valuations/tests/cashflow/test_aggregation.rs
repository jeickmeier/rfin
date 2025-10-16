use finstack_core::{
    currency::Currency,
    dates::{Date, Period, PeriodId},
    money::Money,
};
use finstack_valuations::cashflow::aggregation::aggregate_by_period;
use time::Month;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

fn quarters_2025() -> Vec<Period> {
    vec![
        Period {
            id: PeriodId::quarter(2025, 1),
            start: d(2025, 1, 1),
            end: d(2025, 4, 1),
            is_actual: true,
        },
        Period {
            id: PeriodId::quarter(2025, 2),
            start: d(2025, 4, 1),
            end: d(2025, 7, 1),
            is_actual: false,
        },
        Period {
            id: PeriodId::quarter(2025, 3),
            start: d(2025, 7, 1),
            end: d(2025, 10, 1),
            is_actual: false,
        },
    ]
}

#[test]
fn aggregates_by_period_and_currency() {
    let periods = quarters_2025();
    let flows = vec![
        (d(2025, 4, 15), Money::new(40.0, Currency::USD)),
        (d(2025, 1, 4), Money::new(100.0, Currency::USD)),
        (d(2025, 2, 10), Money::new(150.0, Currency::EUR)),
        (d(2025, 4, 1), Money::new(5.0, Currency::USD)), // boundary → Q2
    ];

    let aggregated = aggregate_by_period(&flows, &periods);
    assert_eq!(aggregated.len(), 2);

    let q1 = aggregated.get(&PeriodId::quarter(2025, 1)).unwrap();
    assert!((q1[&Currency::USD] - 100.0).abs() < 1e-12);
    assert!((q1[&Currency::EUR] - 150.0).abs() < 1e-12);

    let q2 = aggregated.get(&PeriodId::quarter(2025, 2)).unwrap();
    assert!((q2[&Currency::USD] - 45.0).abs() < 1e-12);

    assert!(aggregated.get(&PeriodId::quarter(2025, 3)).is_none());
}

#[test]
fn empty_inputs_return_empty_map() {
    let flows: Vec<(Date, Money)> = vec![];
    let periods = quarters_2025();

    assert!(aggregate_by_period(&flows, &periods).is_empty());
    assert!(
        aggregate_by_period(&[(d(2025, 1, 1), Money::new(1.0, Currency::USD))], &[]).is_empty()
    );
}

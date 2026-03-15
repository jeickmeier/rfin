use finstack_analytics::{multi_factor_greeks, Performance};
use finstack_core::dates::{Date, Month, PeriodKind};

fn d(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).expect("valid date")
}

#[test]
fn standalone_multi_factor_greeks_errors_on_singular_factor_matrix() {
    let returns = [0.02, 0.04, 0.06, 0.08, 0.10];
    let factor_a = [0.01, 0.02, 0.03, 0.04, 0.05];
    let factor_b = [0.02, 0.04, 0.06, 0.08, 0.10];

    let result = multi_factor_greeks(&returns, &[&factor_a, &factor_b], 252.0);
    assert!(
        result.is_err(),
        "singular factor matrices should surface an error rather than a zero-filled result"
    );
}

#[test]
fn performance_multi_factor_greeks_errors_on_invalid_factor_input() {
    let dates = vec![
        d(2024, Month::January, 1),
        d(2024, Month::January, 2),
        d(2024, Month::January, 3),
        d(2024, Month::January, 4),
        d(2024, Month::January, 5),
        d(2024, Month::January, 6),
    ];
    let prices = vec![
        vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0],
        vec![100.0, 100.5, 101.0, 101.5, 102.0, 102.5],
    ];
    let perf = Performance::new(
        dates,
        prices,
        vec!["BENCH".to_string(), "PORT".to_string()],
        Some("BENCH"),
        PeriodKind::Daily,
        false,
    )
    .expect("performance should build");

    let invalid_factor = [0.01, 0.02];
    let result = perf.multi_factor_greeks(1, &[&invalid_factor]);
    assert!(
        result.is_err(),
        "mismatched factor lengths should return an explicit error"
    );
}

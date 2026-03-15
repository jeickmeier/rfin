use finstack_analytics::{group_by_period, Performance};
use finstack_core::dates::{Date, Month, PeriodKind};

fn d(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).expect("valid date")
}

#[test]
fn performance_cagr_uses_full_holding_period_for_single_return_window() {
    let dates = vec![d(2023, Month::January, 1), d(2024, Month::January, 1)];
    let prices = vec![vec![100.0, 110.0]];
    let perf = Performance::new(
        dates,
        prices,
        vec!["PORT".to_string()],
        None,
        PeriodKind::Annual,
        false,
    )
    .expect("performance should build");

    let cagr = perf.cagr();
    assert_eq!(cagr.len(), 1);
    assert!(
        (cagr[0] - 0.10).abs() < 1e-12,
        "expected 10% CAGR over one full year, got {}",
        cagr[0]
    );
}

#[test]
fn cumulative_outperformance_uses_relative_wealth_not_return_difference() {
    let dates = vec![
        d(2024, Month::January, 1),
        d(2024, Month::January, 2),
        d(2024, Month::January, 3),
    ];
    let benchmark_prices = vec![100.0, 80.0, 96.0];
    let portfolio_prices = vec![100.0, 90.0, 108.0];
    let perf = Performance::new(
        dates,
        vec![benchmark_prices, portfolio_prices],
        vec!["BENCH".to_string(), "PORT".to_string()],
        Some("BENCH"),
        PeriodKind::Daily,
        false,
    )
    .expect("performance should build");

    let outperformance = perf.cumulative_returns_outperformance();
    let port_vs_bench = &outperformance[1];
    let expected = [0.125, 0.125];

    assert_eq!(port_vs_bench.len(), expected.len());
    for (actual, expected_value) in port_vs_bench.iter().zip(expected.iter()) {
        assert!(
            (actual - expected_value).abs() < 1e-12,
            "expected compounded relative wealth outperformance {}, got {}",
            expected_value,
            actual
        );
    }
}

#[test]
fn weekly_grouping_uses_iso_week_boundaries_across_year_end() {
    let dates = vec![
        d(2024, Month::December, 30),
        d(2024, Month::December, 31),
        d(2025, Month::January, 2),
    ];
    let returns = vec![0.01, 0.02, 0.03];

    let grouped = group_by_period(&dates, &returns, PeriodKind::Weekly, None);

    assert_eq!(
        grouped.len(),
        1,
        "all observations should fall in one ISO week"
    );
    assert_eq!(grouped[0].0.to_string(), "2025W01");
}

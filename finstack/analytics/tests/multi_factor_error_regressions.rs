use finstack_analytics::benchmark::multi_factor_greeks;
use finstack_analytics::Performance;
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

#[test]
fn standalone_multi_factor_greeks_errors_on_near_singular_factor_matrix() {
    let returns = [0.02, 0.04, 0.06, 0.08, 0.10, 0.12];
    let factor_a = [0.01, 0.02, 0.03, 0.04, 0.05, 0.06];
    let factor_b = [
        0.010_000_000_001,
        0.020_000_000_002,
        0.029_999_999_999,
        0.040_000_000_001,
        0.050_000_000_003,
        0.060_000_000_000,
    ];

    let result = multi_factor_greeks(&returns, &[&factor_a, &factor_b], 252.0);
    assert!(
        result.is_err(),
        "near-singular factor matrices should surface an error instead of unstable coefficients"
    );
}

#[test]
fn standalone_multi_factor_greeks_errors_on_non_positive_ann_factor() {
    let returns = [0.02, 0.04, 0.06, 0.08, 0.10];
    let factor = [0.01, 0.02, 0.03, 0.04, 0.05];

    let zero = multi_factor_greeks(&returns, &[&factor], 0.0);
    let negative = multi_factor_greeks(&returns, &[&factor], -252.0);

    assert!(
        zero.is_err(),
        "zero annualization factors should be rejected explicitly"
    );
    assert!(
        negative.is_err(),
        "negative annualization factors should be rejected explicitly"
    );
}

#[test]
fn standalone_multi_factor_greeks_errors_on_hidden_multicollinearity() {
    let returns = [0.04, 0.01, 0.03, 0.02, 0.05, 0.06];
    let factor_a = [0.01, -0.02, 0.03, -0.01, 0.02, 0.01];
    let factor_b = [0.02, 0.01, -0.01, 0.03, -0.02, 0.04];
    let factor_c: Vec<f64> = factor_a
        .iter()
        .zip(factor_b.iter())
        .map(|(a, b)| a + b)
        .collect();

    let result = multi_factor_greeks(&returns, &[&factor_a, &factor_b, &factor_c], 252.0);

    assert!(
        result.is_err(),
        "factor sets with hidden linear dependence should be rejected explicitly"
    );
}

#[test]
fn standalone_multi_factor_greeks_handles_full_rank_scaled_factors() {
    let factor_a = [1.0e8, 2.0e8, 3.0e8, 4.0e8, 5.0e8, 6.0e8];
    let factor_b = [1.0e-4, -2.0e-4, 3.0e-4, -4.0e-4, 5.0e-4, -6.0e-4];
    let returns: Vec<f64> = factor_a
        .iter()
        .zip(factor_b.iter())
        .map(|(a, b)| 2.0 * a - 3.0 * b)
        .collect();

    let result = multi_factor_greeks(&returns, &[&factor_a, &factor_b], 252.0);
    assert!(
        result.is_ok(),
        "full-rank factor matrices should not be rejected solely because columns live on different scales"
    );

    let result = result.expect("scaled factors should solve successfully");
    assert!(
        (result.betas[0] - 2.0).abs() < 1e-10,
        "expected factor_a beta near 2.0, got {}",
        result.betas[0]
    );
    assert!(
        (result.betas[1] + 3.0).abs() < 5e-5,
        "expected factor_b beta near -3.0, got {}",
        result.betas[1]
    );
}

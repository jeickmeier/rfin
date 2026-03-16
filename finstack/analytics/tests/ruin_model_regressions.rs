use finstack_analytics::{estimate_ruin, Performance, RuinDefinition, RuinModel};
use finstack_core::dates::{Date, Month, PeriodKind};

fn d(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).expect("valid date")
}

#[test]
fn ruin_model_reports_zero_probability_for_strictly_positive_paths() {
    let returns = [0.01; 12];
    let model = RuinModel {
        horizon_periods: 24,
        n_paths: 256,
        block_size: 4,
        seed: 7,
        confidence_level: 0.95,
    };

    let estimate = estimate_ruin(
        &returns,
        RuinDefinition::DrawdownBreach { max_drawdown: 0.20 },
        &model,
    );
    assert_eq!(estimate.probability, 0.0);
}

#[test]
fn ruin_model_reports_certain_ruin_when_every_path_breaches_floor() {
    let returns = [-0.50; 4];
    let model = RuinModel {
        horizon_periods: 1,
        n_paths: 128,
        block_size: 1,
        seed: 11,
        confidence_level: 0.95,
    };

    let estimate = estimate_ruin(
        &returns,
        RuinDefinition::WealthFloor {
            floor_fraction: 0.75,
        },
        &model,
    );
    assert_eq!(estimate.probability, 1.0);
}

#[test]
fn ruin_model_is_reproducible_for_fixed_seed() {
    let returns = [0.03, -0.02, 0.01, -0.04, 0.02, -0.01];
    let model = RuinModel {
        horizon_periods: 18,
        n_paths: 512,
        block_size: 3,
        seed: 42,
        confidence_level: 0.95,
    };

    let first = estimate_ruin(
        &returns,
        RuinDefinition::TerminalFloor {
            floor_fraction: 0.85,
        },
        &model,
    );
    let second = estimate_ruin(
        &returns,
        RuinDefinition::TerminalFloor {
            floor_fraction: 0.85,
        },
        &model,
    );

    assert_eq!(first.probability, second.probability);
    assert_eq!(first.std_err, second.std_err);
    assert_eq!(first.ci_lower, second.ci_lower);
    assert_eq!(first.ci_upper, second.ci_upper);
}

#[test]
fn stricter_ruin_barrier_produces_higher_probability() {
    let returns = [0.04, -0.05, 0.02, -0.03, 0.01, -0.02, 0.03, -0.04];
    let model = RuinModel {
        horizon_periods: 24,
        n_paths: 1024,
        block_size: 2,
        seed: 99,
        confidence_level: 0.95,
    };

    let strict = estimate_ruin(
        &returns,
        RuinDefinition::DrawdownBreach { max_drawdown: 0.10 },
        &model,
    );
    let loose = estimate_ruin(
        &returns,
        RuinDefinition::DrawdownBreach { max_drawdown: 0.25 },
        &model,
    );

    assert!(strict.probability >= loose.probability);
}

#[test]
fn performance_ruin_api_matches_standalone_estimator() {
    let dates = vec![
        d(2024, Month::January, 1),
        d(2024, Month::January, 2),
        d(2024, Month::January, 3),
        d(2024, Month::January, 4),
        d(2024, Month::January, 5),
    ];
    let bench = vec![100.0, 101.0, 99.0, 102.0, 101.0];
    let port = vec![100.0, 103.0, 100.0, 104.0, 102.0];
    let perf = Performance::new(
        dates,
        vec![bench, port],
        vec!["BENCH".to_string(), "PORT".to_string()],
        Some("BENCH"),
        PeriodKind::Daily,
        false,
    )
    .expect("performance should build");

    let model = RuinModel {
        horizon_periods: 12,
        n_paths: 512,
        block_size: 2,
        seed: 123,
        confidence_level: 0.95,
    };
    let definition = RuinDefinition::TerminalFloor {
        floor_fraction: 0.90,
    };

    let standalone = estimate_ruin(
        &[0.03, -0.02912621359223301, 0.04, -0.019230769230769232],
        definition,
        &model,
    );
    let perf_result = perf.estimate_ruin(definition, &model);

    assert_eq!(perf_result.len(), 2);
    assert!((perf_result[1].probability - standalone.probability).abs() < 1e-12);
}

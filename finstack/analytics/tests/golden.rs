use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const SCHEMA_VERSION: &str = "finstack.golden/1";

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    schema_version: String,
    name: String,
    domain: String,
    description: String,
    provenance: Provenance,
    inputs: serde_json::Value,
    expected_outputs: BTreeMap<String, f64>,
    tolerances: BTreeMap<String, ToleranceEntry>,
}

#[derive(Debug, Deserialize)]
struct Provenance {
    as_of: String,
    source: String,
    source_detail: String,
    captured_by: String,
    captured_on: String,
    last_reviewed_by: String,
    last_reviewed_on: String,
    review_interval_months: u32,
    regen_command: String,
}

#[derive(Debug, Deserialize)]
struct ToleranceEntry {
    abs: Option<f64>,
    rel: Option<f64>,
}

fn fixture_path(relative_path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/data")
        .join(relative_path)
}

fn run_golden(relative_path: &str) {
    let path = fixture_path(relative_path);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read fixture {}: {err}", path.display()));
    let fixture: GoldenFixture = serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("parse fixture {}: {err}", path.display()));

    assert_eq!(fixture.schema_version, SCHEMA_VERSION);
    assert!(!fixture.name.trim().is_empty());
    assert!(!fixture.domain.trim().is_empty());
    assert!(!fixture.description.trim().is_empty());
    assert!(!fixture.provenance.as_of.trim().is_empty());
    assert!(!fixture.provenance.source.trim().is_empty());
    assert!(!fixture.provenance.source_detail.trim().is_empty());
    assert!(!fixture.provenance.captured_by.trim().is_empty());
    assert!(!fixture.provenance.captured_on.trim().is_empty());
    assert!(!fixture.provenance.last_reviewed_by.trim().is_empty());
    assert!(!fixture.provenance.last_reviewed_on.trim().is_empty());
    assert!(fixture.provenance.review_interval_months > 0);
    assert!(!fixture.provenance.regen_command.trim().is_empty());

    if fixture.inputs.get("source_validation").is_some() {
        validate_source_validation_fixture(&path, &fixture);
        return;
    }

    if fixture.inputs.get("actual_outputs").is_some() {
        panic!(
            "{} analytics golden requires executable inputs; inputs.actual_outputs is a frozen reference snapshot, not product-code execution",
            path.display()
        );
    }

    let actuals = run_analytics_fixture(&path, &fixture);

    for (metric, expected) in &fixture.expected_outputs {
        let actual = actuals
            .get(metric)
            .copied()
            .unwrap_or_else(|| panic!("{} missing actual metric {metric}", path.display()));
        let tolerance = fixture
            .tolerances
            .get(metric)
            .unwrap_or_else(|| panic!("{} missing tolerance for {metric}", path.display()));
        assert!(
            tolerance.abs.is_some() || tolerance.rel.is_some(),
            "{} tolerance for {metric} has neither abs nor rel",
            path.display()
        );

        let abs_diff = (actual - expected).abs();
        let rel_diff = abs_diff / expected.abs().max(1e-12);
        let abs_ok = tolerance.abs.is_some_and(|abs| abs_diff <= abs);
        let rel_ok = tolerance.rel.is_some_and(|rel| rel_diff <= rel);
        assert!(
            abs_ok || rel_ok,
            "{} metric {metric} actual={actual:.12} expected={expected:.12} abs_diff={abs_diff:.12e} rel_diff={rel_diff:.12e}",
            path.display()
        );
    }
}

fn validate_source_validation_fixture(path: &Path, fixture: &GoldenFixture) {
    let source_validation = &fixture.inputs["source_validation"];
    let status = source_validation
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{} source_validation missing status", path.display()));
    assert_eq!(
        status,
        "non_executable",
        "{} source_validation status must be non_executable",
        path.display()
    );
    let reason = source_validation
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert!(
        !reason.trim().is_empty(),
        "{} source_validation must explain why fixture is non-executable",
        path.display()
    );
    assert!(
        fixture.inputs.get("actual_outputs").is_none(),
        "{} source-validation fixture must not keep inputs.actual_outputs; move frozen references under source_validation.reference_outputs",
        path.display()
    );
    let references = source_validation
        .get("reference_outputs")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| {
            panic!(
                "{} source_validation must retain frozen references under reference_outputs",
                path.display()
            )
        });
    for (metric, expected) in &fixture.expected_outputs {
        let reference = references
            .get(metric)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or_else(|| {
                panic!(
                    "{} source_validation.reference_outputs metric {metric} must exist and be numeric",
                    path.display()
                )
            });
        assert!(
            reference == *expected,
            "{} source_validation.reference_outputs[{metric}]={reference:.17} does not exactly match expected_outputs[{metric}]={expected:.17}",
            path.display()
        );
    }
    for metric in references.keys() {
        assert!(
            fixture.expected_outputs.contains_key(metric),
            "{} source_validation.reference_outputs contains extra metric {metric}",
            path.display()
        );
    }
}

fn run_analytics_fixture(path: &Path, fixture: &GoldenFixture) -> BTreeMap<String, f64> {
    if fixture.inputs.get("computations").is_none() {
        panic!(
            "{} analytics golden has no executable computations; add canonical analytics inputs or mark it as source_validation",
            path.display()
        );
    }
    panic!(
        "{} executable analytics computations are not yet wired for domain {}; source fixtures must be marked source_validation until raw return/price inputs are added",
        path.display(),
        fixture.domain
    );
}

macro_rules! analytics_golden {
    ($name:ident, $path:literal) => {
        #[test]
        #[ignore = "analytics golden fixture data is planned but not landed yet"]
        fn $name() {
            run_golden($path);
        }
    };
}

analytics_golden!(
    golden_returns_log_vs_arith_roundtrip,
    "analytics/returns/log_vs_arith_roundtrip.json"
);
analytics_golden!(
    golden_returns_period_stats_monthly_quarterly_annual,
    "analytics/returns/period_stats_monthly_quarterly_annual.json"
);
analytics_golden!(
    golden_returns_period_stats_weekly_iso,
    "analytics/returns/period_stats_weekly_iso.json"
);
analytics_golden!(
    golden_returns_with_missing_data,
    "analytics/returns/returns_with_missing_data.json"
);
analytics_golden!(
    golden_returns_cumulative_returns,
    "analytics/returns/cumulative_returns.json"
);
analytics_golden!(
    golden_performance_sharpe_known_series,
    "analytics/performance/sharpe_known_series.json"
);
analytics_golden!(
    golden_performance_sortino_known_series,
    "analytics/performance/sortino_known_series.json"
);
analytics_golden!(
    golden_performance_calmar_ratio,
    "analytics/performance/calmar_ratio.json"
);
analytics_golden!(
    golden_performance_information_ratio,
    "analytics/performance/information_ratio.json"
);
analytics_golden!(
    golden_performance_treynor_m2_modsharpe,
    "analytics/performance/treynor_m2_modsharpe.json"
);
analytics_golden!(
    golden_vol_rolling_volatility,
    "analytics/vol/rolling_volatility.json"
);
analytics_golden!(
    golden_vol_ewma_riskmetrics_lambda_94,
    "analytics/vol/ewma_riskmetrics_lambda_94.json"
);
analytics_golden!(
    golden_vol_garch_11_known_series,
    "analytics/vol/garch_11_known_series.json"
);
analytics_golden!(
    golden_drawdown_maxdd_calmar_ulcer,
    "analytics/drawdown/maxdd_calmar_ulcer.json"
);
analytics_golden!(
    golden_drawdown_cdar_chekhlov,
    "analytics/drawdown/cdar_chekhlov.json"
);
analytics_golden!(
    golden_risk_parametric_var_es,
    "analytics/risk/parametric_var_es.json"
);
analytics_golden!(
    golden_risk_historical_var_es,
    "analytics/risk/historical_var_es.json"
);
analytics_golden!(
    golden_risk_cornish_fisher_var,
    "analytics/risk/cornish_fisher_var.json"
);
analytics_golden!(
    golden_benchmark_beta_alpha_regression,
    "analytics/benchmark/beta_alpha_regression.json"
);

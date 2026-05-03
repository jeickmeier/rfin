//! Runner trait, fixture dispatch, and `run_golden!` test macro.

use crate::golden::schema::GoldenFixture;
use crate::golden::tolerance::{compare, ComparisonResult};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const REPORT_HEADER: &str = "runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason\n";
const REPORT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const REPORT_LOCK_POLL: Duration = Duration::from_millis(10);

/// One runner per fixture domain. Runners build canonical API inputs and extract metrics.
pub trait DomainRunner {
    /// Run the canonical computation for this fixture.
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String>;
}

/// Dispatch a fixture to its domain runner by `domain` field.
pub fn dispatch(fixture: &GoldenFixture) -> Result<Box<dyn DomainRunner>, String> {
    match fixture.domain.as_str() {
        "rates.calibration.curves" => Ok(Box::new(
            crate::golden::runners::calibration_curves::CalibrationCurvesRunner,
        )),
        "inflation.calibration.curves" => Ok(Box::new(
            crate::golden::runners::calibration_inflation_curves::CalibrationInflationCurvesRunner,
        )),
        "rates.calibration.swaption_vol" => Ok(Box::new(
            crate::golden::runners::calibration_swaption_vol::CalibrationSwaptionVolRunner,
        )),
        "equity.calibration.vol_smile" | "fx.calibration.vol_smile" => Ok(Box::new(
            crate::golden::runners::calibration_vol_smile::CalibrationVolSmileRunner,
        )),
        "credit.calibration.hazard" => Ok(Box::new(
            crate::golden::runners::calibration_hazard::CalibrationHazardRunner,
        )),
        "rates.integration" => Ok(Box::new(
            crate::golden::runners::integration_rates::IntegrationRatesRunner,
        )),
        "credit.integration" => Ok(Box::new(
            crate::golden::runners::integration_credit::IntegrationCreditRunner,
        )),
        "attribution.equity" | "attribution.fixed_income" => Ok(Box::new(
            crate::golden::runners::attribution_common::AttributionRunner,
        )),
        "fixed_income.bond" => Ok(Box::new(crate::golden::runners::pricing_bond::BondRunner)),
        "fixed_income.bond_future" => Ok(Box::new(
            crate::golden::runners::pricing_bond_future::BondFutureRunner,
        )),
        "fixed_income.convertible" => Ok(Box::new(
            crate::golden::runners::pricing_convertible::ConvertibleRunner,
        )),
        "fixed_income.inflation_linked_bond" => Ok(Box::new(
            crate::golden::runners::pricing_inflation_linked_bond::InflationLinkedBondRunner,
        )),
        "fixed_income.term_loan" => Ok(Box::new(
            crate::golden::runners::pricing_term_loan::TermLoanRunner,
        )),
        "equity.equity_option" => Ok(Box::new(
            crate::golden::runners::pricing_equity_option::EquityOptionRunner,
        )),
        "equity.equity_index_future" => Ok(Box::new(
            crate::golden::runners::pricing_equity_index_future::EquityIndexFutureRunner,
        )),
        "credit.cds" => Ok(Box::new(crate::golden::runners::pricing_cds::CdsRunner)),
        "credit.cds_option" => Ok(Box::new(
            crate::golden::runners::pricing_cds_option::CdsOptionRunner,
        )),
        "credit.cds_tranche" => Ok(Box::new(
            crate::golden::runners::pricing_cds_tranche::CdsTrancheRunner,
        )),
        "fixed_income.structured_credit" => Ok(Box::new(
            crate::golden::runners::pricing_structured_credit::StructuredCreditRunner,
        )),
        "fx.fx_swap" => Ok(Box::new(
            crate::golden::runners::pricing_fx_swap::FxSwapRunner,
        )),
        "fx.fx_option" => Ok(Box::new(
            crate::golden::runners::pricing_fx_option::FxOptionRunner,
        )),
        "rates.cap_floor" => Ok(Box::new(
            crate::golden::runners::pricing_cap_floor::CapFloorRunner,
        )),
        "rates.deposit" => Ok(Box::new(
            crate::golden::runners::pricing_deposit::DepositRunner,
        )),
        "rates.fra" => Ok(Box::new(crate::golden::runners::pricing_fra::FraRunner)),
        "rates.irs" => Ok(Box::new(crate::golden::runners::pricing_irs::IrsRunner)),
        "rates.ir_future" => Ok(Box::new(
            crate::golden::runners::pricing_ir_future::IrFutureRunner,
        )),
        "rates.inflation_swap" => Ok(Box::new(
            crate::golden::runners::pricing_inflation_swap::InflationSwapRunner,
        )),
        "rates.swaption" => Ok(Box::new(
            crate::golden::runners::pricing_swaption::SwaptionRunner,
        )),
        other => Err(format!("no runner registered for domain '{other}'")),
    }
}

/// Run a fixture end-to-end and return one comparison result per expected metric.
pub fn run_fixture(fixture: &GoldenFixture) -> Result<Vec<ComparisonResult>, String> {
    let runner = dispatch(fixture)?;
    let actuals = runner.run(fixture)?;
    fixture
        .expected_outputs
        .iter()
        .map(|(metric, expected)| {
            let actual = actuals
                .get(metric)
                .copied()
                .ok_or_else(|| format!("runner did not produce metric '{metric}'"))?;
            let tolerance = fixture
                .tolerances
                .get(metric)
                .ok_or_else(|| format!("no tolerance for metric '{metric}'"))?;
            Ok(compare(metric, actual, *expected, tolerance))
        })
        .collect()
}

fn non_compared_metric_reason(fixture: &GoldenFixture, metric: &str) -> Option<String> {
    let _ = (fixture, metric);
    None
}

/// Run one golden fixture from disk, write a CSV comparison report, and return failures.
pub fn run_golden_at_path(path: &Path) -> Result<Vec<ComparisonResult>, String> {
    crate::golden::walk::validate_fixture(path)
        .map_err(|err| format!("validate fixture {path:?}: {err}"))?;
    let raw =
        std::fs::read_to_string(path).map_err(|err| format!("read fixture {path:?}: {err}"))?;
    let fixture: GoldenFixture =
        serde_json::from_str(&raw).map_err(|err| format!("parse fixture {path:?}: {err}"))?;
    let results = run_fixture(&fixture)?;
    write_comparison_csv(path, &results)?;
    Ok(results)
}

fn write_comparison_csv(path: &Path, results: &[ComparisonResult]) -> Result<(), String> {
    let report_path = comparison_report_path();
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create report directory {parent:?}: {err}"))?;
    }

    let fixture = fixture_relative_path(path)?;
    let _lock = ReportLock::acquire(&report_path)?;
    let mut rows = existing_comparison_rows(&report_path, "rust", &fixture)?;
    for result in results {
        rows.push(format!(
            "{},{},{},{:.12},{:.12},{:.12e},{:.12e},{},{},{},{}",
            "rust",
            csv_escape(&fixture),
            csv_escape(&result.metric),
            result.actual,
            result.expected,
            result.abs_diff,
            result.rel_diff,
            optional_f64(result.used_tolerance.abs),
            optional_f64(result.used_tolerance.rel),
            result.passed,
            csv_escape(
                result
                    .used_tolerance
                    .tolerance_reason
                    .as_deref()
                    .unwrap_or("")
            ),
        ));
    }

    let mut csv = String::from(REPORT_HEADER);
    csv.push_str(&rows.join("\n"));
    if !rows.is_empty() {
        csv.push('\n');
    }

    write_report_atomically(&report_path, csv)
}

fn comparison_report_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../target/golden-reports/golden-comparisons.csv")
}

fn existing_comparison_rows(
    report_path: &Path,
    runner: &str,
    fixture: &str,
) -> Result<Vec<String>, String> {
    if !report_path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(report_path)
        .map_err(|err| format!("read comparison report {report_path:?}: {err}"))?;
    Ok(raw
        .lines()
        .skip(1)
        .filter(|line| !line.starts_with(&format!("{runner},{fixture},")))
        .map(str::to_string)
        .collect())
}

struct ReportLock {
    path: PathBuf,
}

impl ReportLock {
    fn acquire(report_path: &Path) -> Result<Self, String> {
        let lock_path = report_path.with_extension("csv.lock");
        let deadline = Instant::now() + REPORT_LOCK_TIMEOUT;

        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(Self { path: lock_path }),
                Err(err) if err.kind() == ErrorKind::AlreadyExists && Instant::now() < deadline => {
                    thread::sleep(REPORT_LOCK_POLL);
                }
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    return Err(format!("timed out waiting for report lock {lock_path:?}"));
                }
                Err(err) => return Err(format!("create report lock {lock_path:?}: {err}")),
            }
        }
    }
}

impl Drop for ReportLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn write_report_atomically(report_path: &Path, csv: String) -> Result<(), String> {
    let temp_path = report_path.with_extension(format!("csv.{}.tmp", std::process::id()));
    std::fs::write(&temp_path, csv)
        .map_err(|err| format!("write temporary comparison report {temp_path:?}: {err}"))?;
    std::fs::rename(&temp_path, report_path).map_err(|err| {
        let _ = std::fs::remove_file(&temp_path);
        format!("replace comparison report {report_path:?}: {err}")
    })
}

fn fixture_relative_path(path: &Path) -> Result<String, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = path
        .strip_prefix(manifest_dir.join("tests/golden/data"))
        .map_err(|err| format!("fixture path {path:?} is outside tests/golden/data: {err}"))?;
    Ok(relative.to_string_lossy().to_string())
}

fn optional_f64(value: Option<f64>) -> String {
    value.map_or_else(String::new, |v| format!("{v:.12}"))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Run one golden fixture from a path relative to `tests/golden/data/`.
#[macro_export]
macro_rules! run_golden {
    ($relative_path:expr) => {{
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir)
            .join("tests/golden/data")
            .join($relative_path);
        let results = $crate::golden::runner::run_golden_at_path(&path)
            .unwrap_or_else(|err| panic!("run fixture {:?}: {}", path, err));
        let failures = results
            .iter()
            .filter(|result| !result.passed)
            .map(|result| result.failure_message(&path.display().to_string()))
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            panic!(
                "{} metric(s) failed:\n{}",
                failures.len(),
                failures.join("\n\n")
            );
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::schema::{Provenance, ToleranceEntry};

    #[test]
    fn run_golden_at_path_writes_comparison_csv() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path =
            Path::new(manifest_dir).join("tests/golden/data/pricing/deposit/usd_deposit_3m.json");
        let report_path =
            Path::new(manifest_dir).join("../../target/golden-reports/golden-comparisons.csv");

        run_golden_at_path(&fixture_path).expect("golden should pass and write CSV report");

        let csv = std::fs::read_to_string(&report_path).expect("CSV report should exist");
        assert!(csv.contains("runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason"));
        assert!(csv.contains("rust,pricing/deposit/usd_deposit_3m.json,npv,"));
        assert!(csv.contains(",true,"));
    }

    #[test]
    fn run_golden_at_path_validates_fixture_before_dispatch() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path =
            Path::new(manifest_dir).join("../../target/golden-test-invalid-schema.json");
        std::fs::write(
            &fixture_path,
            r#"{
              "schema_version": "finstack.golden/0",
              "name": "invalid_schema",
              "domain": "rates.irs",
              "description": "Fixture with stale schema version.",
              "provenance": {
                "as_of": "2026-04-30",
                "source": "formula",
                "source_detail": "unit test",
                "captured_by": "test",
                "captured_on": "2026-04-30",
                "last_reviewed_by": "test",
                "last_reviewed_on": "2026-04-30",
                "review_interval_months": 6,
                "regen_command": "",
                "screenshots": []
              },
              "inputs": {},
              "expected_outputs": {"npv": 0.0},
              "tolerances": {"npv": {"abs": 0.0}}
            }"#,
        )
        .expect("write invalid fixture");

        let err = run_golden_at_path(&fixture_path).expect_err("fixture validation should fail");

        assert!(
            err.contains("schema_version is 'finstack.golden/0'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn flattened_output_placeholder_runner_is_not_false_green() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "placeholder".to_string(),
            domain: "rates.calibration.curves".to_string(),
            description: "Placeholder fixture".to_string(),
            provenance: Provenance {
                as_of: "2026-04-30".to_string(),
                source: "formula".to_string(),
                source_detail: "unit test".to_string(),
                captured_by: "test".to_string(),
                captured_on: "2026-04-30".to_string(),
                last_reviewed_by: "test".to_string(),
                last_reviewed_on: "2026-04-30".to_string(),
                review_interval_months: 6,
                regen_command: String::new(),
                screenshots: Vec::new(),
            },
            inputs: serde_json::json!({"actual_outputs": {"calibration_rmse": 0.0}}),
            expected_outputs: BTreeMap::from([("calibration_rmse".to_string(), 0.0)]),
            tolerances: BTreeMap::from([(
                "calibration_rmse".to_string(),
                ToleranceEntry {
                    abs: Some(0.0),
                    rel: None,
                    tolerance_reason: None,
                },
            )]),
        };

        let err = crate::golden::runners::reject_flattened_outputs("placeholder runner", &fixture)
            .expect_err("placeholder helper should fail");

        assert!(
            err.contains("requires executable inputs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn attribution_raw_looking_keys_do_not_bypass_execution_requirement() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "attribution_placeholder".to_string(),
            domain: "attribution.equity".to_string(),
            description: "Attribution placeholder fixture".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "components": {"selection::tech": 0.01},
                "sums": {"total_active": ["selection::tech"]},
                "holdings": []
            }),
            expected_outputs: BTreeMap::from([("total_active".to_string(), 0.01)]),
            tolerances: BTreeMap::from([("total_active".to_string(), abs_zero())]),
        };

        let err = run_fixture(&fixture).expect_err("non-source attribution must reject");

        assert!(
            err.contains("requires executable inputs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn source_validation_reference_values_must_match_expected_outputs() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "bad_source_validation".to_string(),
            domain: "attribution.equity".to_string(),
            description: "Source validation mismatch fixture".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "components": {"selection::tech": 0.01},
                "source_validation": {
                    "status": "non_executable",
                    "reason": "unit test",
                    "reference_outputs": {"selection::tech": 0.02}
                }
            }),
            expected_outputs: BTreeMap::from([("selection::tech".to_string(), 0.01)]),
            tolerances: BTreeMap::from([("selection::tech".to_string(), abs_zero())]),
        };

        let err = run_fixture(&fixture).expect_err("source reference mismatch must fail");

        assert!(
            err.contains("does not exactly match expected_outputs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pricing_source_validation_still_dispatches_to_runner() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "source_validation_pricing_dispatch".to_string(),
            domain: "rates.deposit".to_string(),
            description: "Source validation pricing dispatch test".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "source_validation": {
                    "status": "non_executable",
                    "reason": "unit test",
                    "reference_outputs": {"npv": 0.0}
                }
            }),
            expected_outputs: BTreeMap::from([("npv".to_string(), 0.0)]),
            tolerances: BTreeMap::from([("npv".to_string(), abs_zero())]),
        };

        let err = run_fixture(&fixture).expect_err("pricing source validation must execute runner");

        assert!(
            err.contains("parse pricing inputs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn source_validation_requires_reason_before_reference_comparison() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "source_validation_missing_reason".to_string(),
            domain: "attribution.equity".to_string(),
            description: "Source validation reason test".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "components": {"selection::tech": 0.01},
                "source_validation": {
                    "status": "non_executable",
                    "reference_outputs": {"selection::tech": 0.01}
                }
            }),
            expected_outputs: BTreeMap::from([("selection::tech".to_string(), 0.01)]),
            tolerances: BTreeMap::from([("selection::tech".to_string(), abs_zero())]),
        };

        let err = run_fixture(&fixture).expect_err("source validation must explain non-execution");

        assert!(err.contains("must explain"), "unexpected error: {err}");
    }

    #[test]
    fn source_validation_rejects_legacy_actual_outputs() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "source_validation_actual_outputs".to_string(),
            domain: "attribution.equity".to_string(),
            description: "Source validation actual_outputs test".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "actual_outputs": {"selection::tech": 0.01},
                "components": {"selection::tech": 0.01},
                "source_validation": {
                    "status": "non_executable",
                    "reason": "unit test",
                    "reference_outputs": {"selection::tech": 0.01}
                }
            }),
            expected_outputs: BTreeMap::from([("selection::tech".to_string(), 0.01)]),
            tolerances: BTreeMap::from([("selection::tech".to_string(), abs_zero())]),
        };

        let err = run_fixture(&fixture).expect_err("source validation must reject actual_outputs");

        assert!(
            err.contains("must not keep inputs.actual_outputs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn source_validation_fixture_is_non_gating_and_writes_reference_rows() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = Path::new(manifest_dir)
            .join("tests/golden/data/attribution/brinson_hood_beebower.json");
        let report_path =
            Path::new(manifest_dir).join("../../target/golden-reports/golden-comparisons.csv");

        let results = run_golden_at_path(&fixture_path).expect("source validation should validate");

        assert!(!results.is_empty());
        let csv = std::fs::read_to_string(&report_path).expect("CSV report should exist");
        assert!(csv.contains("rust,attribution/brinson_hood_beebower.json,total_active,"));
        assert!(csv.contains("rust,attribution/brinson_hood_beebower.json,allocation::energy,"));
        assert!(csv.contains(",true,"));
    }

    #[test]
    fn required_pricing_risk_metric_cannot_be_non_compared() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "bad_non_compared_required_metric".to_string(),
            domain: "credit.cds_tranche".to_string(),
            description: "Required metric bypass test".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "source_reference": {
                    "non_compared_metrics": ["cs01"],
                    "non_compared_metrics_reason": "unit test"
                }
            }),
            expected_outputs: BTreeMap::from([
                ("cs01".to_string(), 1.0),
                ("dv01".to_string(), 2.0),
            ]),
            tolerances: BTreeMap::from([
                ("cs01".to_string(), abs_zero()),
                ("dv01".to_string(), abs_zero()),
            ]),
        };

        assert!(non_compared_metric_reason(&fixture, "cs01").is_none());
    }

    #[test]
    fn source_reference_non_compared_metric_does_not_bypass_comparison() {
        let fixture = GoldenFixture {
            schema_version: crate::golden::schema::SCHEMA_VERSION.to_string(),
            name: "bad_non_compared_optional_metric".to_string(),
            domain: "rates.irs".to_string(),
            description: "Optional metric bypass test".to_string(),
            provenance: test_provenance(),
            inputs: serde_json::json!({
                "source_reference": {
                    "non_compared_metrics": ["npv"],
                    "non_compared_metrics_reason": "unit test"
                }
            }),
            expected_outputs: BTreeMap::from([("npv".to_string(), 1.0), ("dv01".to_string(), 2.0)]),
            tolerances: BTreeMap::from([
                ("npv".to_string(), abs_zero()),
                ("dv01".to_string(), abs_zero()),
            ]),
        };

        assert!(non_compared_metric_reason(&fixture, "npv").is_none());
    }

    fn test_provenance() -> Provenance {
        Provenance {
            as_of: "2026-04-30".to_string(),
            source: "formula".to_string(),
            source_detail: "unit test".to_string(),
            captured_by: "test".to_string(),
            captured_on: "2026-04-30".to_string(),
            last_reviewed_by: "test".to_string(),
            last_reviewed_on: "2026-04-30".to_string(),
            review_interval_months: 6,
            regen_command: String::new(),
            screenshots: Vec::new(),
        }
    }

    fn abs_zero() -> ToleranceEntry {
        ToleranceEntry {
            abs: Some(0.0),
            rel: None,
            tolerance_reason: None,
        }
    }
}

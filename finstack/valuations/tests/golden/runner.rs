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
        "rates.calibration.swaption_vol" => Ok(Box::new(
            crate::golden::runners::calibration_swaption_vol::CalibrationSwaptionVolRunner,
        )),
        "rates.integration" => Ok(Box::new(
            crate::golden::runners::integration_rates::IntegrationRatesRunner,
        )),
        "fx.fx_swap" => Ok(Box::new(
            crate::golden::runners::pricing_fx_swap::FxSwapRunner,
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

/// Run one golden fixture from disk, write a CSV comparison report, and return failures.
pub fn run_golden_at_path(path: &Path) -> Result<Vec<ComparisonResult>, String> {
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

    #[test]
    fn run_golden_at_path_writes_comparison_csv() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = Path::new(manifest_dir)
            .join("tests/golden/data/pricing/irs/usd_sofr_5y_receive_fixed_swpm.json");
        let report_path =
            Path::new(manifest_dir).join("../../target/golden-reports/golden-comparisons.csv");

        run_golden_at_path(&fixture_path).expect("golden should pass and write CSV report");

        let csv = std::fs::read_to_string(&report_path).expect("CSV report should exist");
        assert!(csv.contains("runner,fixture,metric,actual,expected,abs_diff,rel_diff,abs_tolerance,rel_tolerance,passed,tolerance_reason"));
        assert!(csv.contains("rust,pricing/irs/usd_sofr_5y_receive_fixed_swpm.json,npv,"));
        assert!(csv.contains(",true,"));
    }
}

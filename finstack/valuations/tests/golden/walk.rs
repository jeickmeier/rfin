//! Walk-test for validating every committed golden fixture.

use crate::golden::schema::{GoldenFixture, SCHEMA_VERSION};
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::pricer::parse_boxed_instrument_json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DATA_ROOT: &str = "tests/golden/data";
const MANUAL_SCREENSHOT_SOURCES: &[&str] = &["bloomberg-screen", "intex"];
const VALID_SOURCES: &[&str] = &[
    "quantlib",
    "bloomberg-api",
    "bloomberg-screen",
    "intex",
    "formula",
    "textbook",
];
const PRICING_INPUT_KEYS: &[&str] = &[
    "valuation_date",
    "model",
    "metrics",
    "instrument_json",
    "market",
    "source_reference",
];
const PRICING_OPTIONAL_INPUT_KEYS: &[&str] = &["source_validation"];
const ZERO_RISK_METRICS_REQUIRING_REASON: &[&str] = &[
    "bucketed_dv01",
    "convexity",
    "cs01",
    "delta",
    "duration_mod",
    "dv01",
    "foreign_rho",
    "gamma",
    "inflation01",
    "recovery_01",
    "rho",
    "spread_dv01",
    "vega",
];

fn collect_fixture_paths() -> Vec<PathBuf> {
    collect_fixture_paths_from(&data_root())
}

pub(crate) fn collect_fixture_paths_under(relative_dir: &str) -> Result<Vec<PathBuf>, String> {
    let paths = collect_fixture_paths_from(&data_root().join(relative_dir));
    let read_errors = paths
        .iter()
        .filter(|path| path.to_string_lossy().starts_with("__read_dir_error__:"))
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if read_errors.is_empty() {
        Ok(paths)
    } else {
        Err(read_errors.join("\n"))
    }
}

fn data_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join(DATA_ROOT)
}

fn collect_fixture_paths_from(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    walk_dir(root, &mut paths);
    paths.sort();
    paths
}

fn walk_dir(dir: &Path, paths: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            paths.push(PathBuf::from(format!(
                "__read_dir_error__:{}:{}",
                dir.display(),
                err
            )));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("screenshots") {
                walk_dir(&path, paths);
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
}

pub(crate) fn validate_fixture(path: &Path) -> Result<(), String> {
    if path.to_string_lossy().starts_with("__read_dir_error__:") {
        return Err(path.display().to_string());
    }

    let raw = fs::read_to_string(path).map_err(|err| format!("read failed: {err}"))?;
    let fixture: GoldenFixture =
        serde_json::from_str(&raw).map_err(|err| format!("parse failed: {err}"))?;

    if fixture.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "schema_version is '{}', expected '{}'",
            fixture.schema_version, SCHEMA_VERSION
        ));
    }

    if !VALID_SOURCES.contains(&fixture.provenance.source.as_str()) {
        return Err(format!(
            "provenance.source '{}' is not recognized",
            fixture.provenance.source
        ));
    }

    validate_non_empty("name", &fixture.name)?;
    validate_non_empty("domain", &fixture.domain)?;
    validate_non_empty("description", &fixture.description)?;
    validate_non_empty("provenance.as_of", &fixture.provenance.as_of)?;
    validate_non_empty(
        "provenance.source_detail",
        &fixture.provenance.source_detail,
    )?;
    validate_non_empty("provenance.captured_by", &fixture.provenance.captured_by)?;
    validate_non_empty("provenance.captured_on", &fixture.provenance.captured_on)?;
    validate_non_empty(
        "provenance.last_reviewed_by",
        &fixture.provenance.last_reviewed_by,
    )?;
    validate_non_empty(
        "provenance.last_reviewed_on",
        &fixture.provenance.last_reviewed_on,
    )?;

    for metric in fixture.expected_outputs.keys() {
        if !fixture.tolerances.contains_key(metric) {
            return Err(format!(
                "expected_outputs has '{}' but tolerances does not",
                metric
            ));
        }
    }

    for (metric, tolerance) in &fixture.tolerances {
        if !fixture.expected_outputs.contains_key(metric) {
            return Err(format!(
                "tolerances has '{}' but expected_outputs does not",
                metric
            ));
        }
        if tolerance.abs.is_none() && tolerance.rel.is_none() {
            return Err(format!(
                "tolerance for '{}' has neither abs nor rel",
                metric
            ));
        }
    }

    validate_zero_risk_metric_reasons(&fixture)?;
    validate_source_reference_coverage(&fixture)?;
    validate_source_validation_metadata(&fixture)?;
    validate_pricing_input_schema(path, &fixture)?;
    validate_required_pricing_risk_metrics(&fixture)?;
    validate_required_metrics_not_non_compared(&fixture)?;

    if MANUAL_SCREENSHOT_SOURCES.contains(&fixture.provenance.source.as_str())
        && fixture.provenance.screenshots.is_empty()
    {
        return Err(format!(
            "source '{}' requires at least one screenshot",
            fixture.provenance.source
        ));
    }

    let parent = path.parent().ok_or("fixture has no parent dir")?;
    for shot in &fixture.provenance.screenshots {
        let shot_path = parent.join(&shot.path);
        if !shot_path.exists() {
            return Err(format!(
                "screenshot '{}' does not exist (resolved to {:?})",
                shot.path, shot_path
            ));
        }
        if !is_git_tracked(&shot_path) {
            return Err(format!(
                "screenshot '{}' exists but is not tracked by git",
                shot.path
            ));
        }
    }

    Ok(())
}

fn validate_pricing_input_schema(path: &Path, fixture: &GoldenFixture) -> Result<(), String> {
    let Ok(relative) = path.strip_prefix(data_root()) else {
        return Ok(());
    };
    if !relative.to_string_lossy().starts_with("pricing/") {
        return Ok(());
    }

    let inputs = fixture
        .inputs
        .as_object()
        .ok_or("pricing fixture inputs must be an object")?;
    validate_object_keys(
        "inputs",
        inputs,
        PRICING_INPUT_KEYS,
        PRICING_OPTIONAL_INPUT_KEYS,
    )?;

    let market = inputs
        .get("market")
        .ok_or("pricing fixture inputs.market is required")?;
    serde_json::from_value::<MarketContext>(market.clone()).map_err(|err| {
        format!("pricing fixture inputs.market is not a valid MarketContext: {err}")
    })?;

    let instrument_json = inputs
        .get("instrument_json")
        .ok_or("pricing fixture inputs.instrument_json is required")?;
    let instrument_json = serde_json::to_string(instrument_json)
        .map_err(|err| format!("serialize pricing fixture inputs.instrument_json: {err}"))?;
    parse_boxed_instrument_json(&instrument_json, None).map_err(|err| {
        format!("pricing fixture inputs.instrument_json is not a valid instrument: {err}")
    })?;

    let metrics = string_array(inputs, "metrics")?;
    for metric in &metrics {
        MetricId::parse_strict(metric)
            .map_err(|err| format!("pricing fixture inputs.metrics contains '{metric}': {err}"))?;
    }
    let requested = metrics.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for metric in fixture.expected_outputs.keys() {
        if metric != "npv" && !requested.contains(metric.as_str()) {
            return Err(format!(
                "expected_outputs has '{metric}' but inputs.metrics does not request it"
            ));
        }
    }

    Ok(())
}

fn validate_object_keys(
    field: &str,
    object: &serde_json::Map<String, serde_json::Value>,
    required: &[&str],
    optional: &[&str],
) -> Result<(), String> {
    let allowed = required
        .iter()
        .chain(optional.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    for key in object.keys() {
        if !allowed.contains(key.as_str()) {
            return Err(format!("{field} has unexpected key '{key}'"));
        }
    }
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("{field} is missing required key '{key}'"));
        }
    }
    Ok(())
}

fn validate_required_pricing_risk_metrics(fixture: &GoldenFixture) -> Result<(), String> {
    if fixture.domain.contains(".integration") || fixture.domain.contains(".calibration.") {
        return Ok(());
    }

    if fixture.domain.starts_with("rates.")
        && fixture.domain != "rates.integration"
        && !fixture.domain.starts_with("rates.calibration.")
        && !fixture.expected_outputs.contains_key("dv01")
    {
        return Err("rates pricing fixtures must assert dv01".to_string());
    }

    if fixture.domain.starts_with("fixed_income.") && !fixture.expected_outputs.contains_key("dv01")
    {
        return Err("fixed-income pricing fixtures must assert dv01".to_string());
    }

    if fixture.domain.starts_with("credit.") {
        if !fixture.expected_outputs.contains_key("dv01") {
            return Err("credit pricing fixtures must assert dv01".to_string());
        }
        if !fixture.expected_outputs.contains_key("cs01") {
            return Err("credit pricing fixtures must assert cs01".to_string());
        }
    }

    Ok(())
}

fn validate_required_metrics_not_non_compared(fixture: &GoldenFixture) -> Result<(), String> {
    let Some(source_reference) = fixture
        .inputs
        .get("source_reference")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(());
    };
    let non_compared = string_array(source_reference, "non_compared_metrics")?;
    let invalid = non_compared
        .iter()
        .filter(|metric| is_required_executable_pricing_risk_metric(fixture, metric))
        .cloned()
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "required executable pricing/risk metrics cannot be listed in inputs.source_reference.non_compared_metrics: {}",
            invalid.join(", ")
        ))
    }
}

fn is_required_executable_pricing_risk_metric(fixture: &GoldenFixture, metric: &str) -> bool {
    if fixture.domain.contains(".integration") || fixture.domain.contains(".calibration.") {
        return false;
    }
    if fixture.domain.starts_with("rates.") {
        return metric == "dv01";
    }
    if fixture.domain.starts_with("fixed_income.") {
        return metric == "dv01";
    }
    fixture.domain.starts_with("credit.") && matches!(metric, "dv01" | "cs01")
}

fn validate_source_validation_metadata(fixture: &GoldenFixture) -> Result<(), String> {
    if fixture.inputs.get("source_validation").is_none() {
        return Ok(());
    }
    crate::golden::runners::validate_source_validation_fixture("walk validation", fixture)
        .map(|_| ())
}

fn validate_zero_risk_metric_reasons(fixture: &GoldenFixture) -> Result<(), String> {
    for (metric, expected) in &fixture.expected_outputs {
        if expected.abs() <= f64::EPSILON
            && ZERO_RISK_METRICS_REQUIRING_REASON.contains(&metric.as_str())
            && !has_zero_metric_reason(fixture, metric)
        {
            return Err(format!(
                "zero risk metric '{metric}' requires a tolerance_reason or inputs.source_reference.zero_metric_reasons entry"
            ));
        }
    }
    Ok(())
}

fn has_zero_metric_reason(fixture: &GoldenFixture, metric: &str) -> bool {
    if fixture
        .tolerances
        .get(metric)
        .and_then(|tolerance| tolerance.tolerance_reason.as_deref())
        .is_some_and(|reason| !reason.trim().is_empty())
    {
        return true;
    }
    fixture
        .inputs
        .get("source_reference")
        .and_then(serde_json::Value::as_object)
        .and_then(|source_reference| source_reference.get("zero_metric_reasons"))
        .and_then(serde_json::Value::as_object)
        .and_then(|reasons| reasons.get(metric))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|reason| !reason.trim().is_empty())
}

fn validate_source_reference_coverage(fixture: &GoldenFixture) -> Result<(), String> {
    let Some(source_reference) = fixture.inputs.get("source_reference") else {
        return Ok(());
    };
    let Some(source_reference) = source_reference.as_object() else {
        return Err("inputs.source_reference must be an object".to_string());
    };

    let planned = string_array(source_reference, "planned_metrics_not_compared")?;
    let non_compared = string_array(source_reference, "non_compared_metrics")?;
    if (!planned.is_empty() || !non_compared.is_empty())
        && !has_metric_omission_reason(source_reference)
    {
        return Err(
            "inputs.source_reference planned/non-compared metrics require an explicit reason"
                .to_string(),
        );
    }

    let expected = fixture
        .expected_outputs
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let omitted = planned
        .iter()
        .chain(non_compared.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for metric in string_array(source_reference, "design_metrics")? {
        let aliases = design_metric_aliases(source_reference, &metric);
        let alias_asserted = aliases
            .iter()
            .any(|alias| expected.contains(alias.as_str()));
        let alias_omitted = aliases.iter().any(|alias| omitted.contains(alias.as_str()));
        if !expected.contains(metric.as_str())
            && !omitted.contains(metric.as_str())
            && !alias_asserted
            && !alias_omitted
        {
            return Err(format!(
                "inputs.source_reference design metric '{metric}' is neither asserted nor listed as planned/non-compared"
            ));
        }
    }

    Ok(())
}

fn string_array(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(format!("inputs.source_reference.{key} must be an array"));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("inputs.source_reference.{key} entries must be strings"))
        })
        .collect()
}

fn has_metric_omission_reason(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    [
        "planned_metrics_reason",
        "non_compared_metrics_reason",
        "omission_reason",
        "delta_convention_note",
        "waterfall_reference",
        "note",
    ]
    .iter()
    .any(|key| {
        object
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn design_metric_aliases(
    object: &serde_json::Map<String, serde_json::Value>,
    metric: &str,
) -> Vec<String> {
    let mut aliases = Vec::new();
    let key = format!("{metric}_key");
    if let Some(alias) = object.get(&key).and_then(serde_json::Value::as_str) {
        aliases.push(alias.to_string());
    }
    if metric == "mod_duration" {
        if let Some(alias) = object
            .get("duration_key")
            .and_then(serde_json::Value::as_str)
        {
            aliases.push(alias.to_string());
        }
    }
    if let Some(strict_metric_keys) = object
        .get("strict_metric_keys")
        .and_then(serde_json::Value::as_object)
    {
        if let Some(alias) = strict_metric_keys
            .get(metric)
            .and_then(serde_json::Value::as_str)
        {
            aliases.push(alias.to_string());
        }
    }
    aliases
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} is empty"));
    }
    Ok(())
}

fn is_git_tracked(path: &Path) -> bool {
    Command::new("git")
        .arg("ls-files")
        .arg("--error-unmatch")
        .arg(path)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn fixture_relative_path(path: &Path) -> Result<String, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let data_root = Path::new(manifest_dir).join(DATA_ROOT);
    path.strip_prefix(data_root)
        .map(|relative| relative.to_string_lossy().to_string())
        .map_err(|err| format!("fixture path {path:?} is outside {DATA_ROOT}: {err}"))
}

fn collect_declared_run_golden_paths() -> Result<BTreeSet<String>, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let golden_root = Path::new(manifest_dir).join("tests/golden");
    let mut declared = BTreeSet::new();
    for source in [
        "pricing.rs",
        "calibration.rs",
        "integration.rs",
        "attribution.rs",
    ] {
        let path = golden_root.join(source);
        let raw = fs::read_to_string(&path).map_err(|err| format!("read {path:?}: {err}"))?;
        for line in raw.lines() {
            if let Some(relative) = extract_run_golden_path(line) {
                declared.insert(relative);
            }
        }
    }
    Ok(declared)
}

fn extract_run_golden_path(line: &str) -> Option<String> {
    let start = line.find("run_golden!(\"")? + "run_golden!(\"".len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEPOSIT_FIXTURE: &str = "pricing/deposit/usd_deposit_3m.json";

    #[test]
    fn pricing_input_schema_rejects_invalid_instrument_json() {
        let path = data_root().join(DEPOSIT_FIXTURE);
        let mut fixture = load_fixture(DEPOSIT_FIXTURE);
        fixture.inputs["instrument_json"] = serde_json::json!({
            "schema": "finstack.instrument/1",
            "instrument": {
                "type": "deposit",
                "spec": {}
            }
        });

        let err = validate_pricing_input_schema(&path, &fixture)
            .expect_err("invalid instrument_json must fail pricing walk validation");

        assert!(
            err.contains("instrument_json"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pricing_input_schema_rejects_unknown_metric_name() {
        let path = data_root().join(DEPOSIT_FIXTURE);
        let mut fixture = load_fixture(DEPOSIT_FIXTURE);
        fixture.inputs["metrics"] = serde_json::json!(["deposit_par_rate", "dv01x"]);

        let err = validate_pricing_input_schema(&path, &fixture)
            .expect_err("unknown metric names must fail pricing walk validation");

        assert!(err.contains("dv01x"), "unexpected error: {err}");
    }

    #[test]
    fn pricing_input_schema_requires_expected_metrics_to_be_requested() {
        let path = data_root().join(DEPOSIT_FIXTURE);
        let mut fixture = load_fixture(DEPOSIT_FIXTURE);
        fixture.inputs["metrics"] = serde_json::json!(["deposit_par_rate"]);

        let err = validate_pricing_input_schema(&path, &fixture)
            .expect_err("expected risk metrics must be requested in pricing inputs");

        assert!(err.contains("dv01"), "unexpected error: {err}");
    }

    #[test]
    fn source_validation_does_not_allow_required_metric_as_non_compared() {
        let mut fixture = load_fixture(DEPOSIT_FIXTURE);
        fixture.inputs["source_validation"] = serde_json::json!({
            "status": "non_executable",
            "reason": "unit test",
            "reference_outputs": fixture.expected_outputs
        });
        fixture.inputs["source_reference"]["non_compared_metrics"] = serde_json::json!(["dv01"]);
        fixture.inputs["source_reference"]["non_compared_metrics_reason"] =
            serde_json::json!("unit test");

        let err = validate_required_metrics_not_non_compared(&fixture)
            .expect_err("source_validation must not hide required executable risk metrics");

        assert!(err.contains("dv01"), "unexpected error: {err}");
    }

    #[test]
    fn source_validation_metadata_requires_reason() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = manifest_dir.join("../../target/golden-source-validation-missing-reason.json");
        std::fs::write(
            &path,
            r#"{
              "schema_version": "finstack.golden/1",
              "name": "missing_source_validation_reason",
              "domain": "attribution.equity",
              "description": "Source validation missing reason test.",
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
              "inputs": {
                "components": {"selection::tech": 0.01},
                "source_validation": {
                  "status": "non_executable",
                  "reference_outputs": {"selection::tech": 0.01}
                }
              },
              "expected_outputs": {"selection::tech": 0.01},
              "tolerances": {"selection::tech": {"abs": 0.0}}
            }"#,
        )
        .expect("write source validation fixture");

        let err = validate_fixture(&path).expect_err("source_validation reason is required");

        assert!(err.contains("must explain"), "unexpected error: {err}");
    }

    fn load_fixture(relative_path: &str) -> GoldenFixture {
        let raw = fs::read_to_string(data_root().join(relative_path)).expect("read fixture");
        serde_json::from_str(&raw).expect("parse fixture")
    }
}

#[test]
fn all_fixtures_well_formed() {
    let failures = collect_fixture_paths()
        .iter()
        .filter_map(|path| {
            validate_fixture(path).err().map(|msg| {
                if msg.starts_with("__read_dir_error__:") {
                    msg
                } else {
                    format!("{}: {}", path.display(), msg)
                }
            })
        })
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "{} fixture(s) failed validation:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn all_fixtures_are_declared_in_rust_golden_tests() {
    let declared = collect_declared_run_golden_paths().expect("collect run_golden declarations");
    let failures = collect_fixture_paths()
        .iter()
        .filter_map(|path| match fixture_relative_path(path) {
            Ok(relative) if relative.starts_with("pricing/") => None,
            Ok(relative) if relative.starts_with("integration/") => None,
            Ok(relative) if declared.contains(&relative) => None,
            Ok(relative) => Some(format!("missing run_golden! declaration for {relative}")),
            Err(err) => Some(err),
        })
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "{} fixture(s) are not declared in Rust golden tests:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn pricing_fixture_discovery_uses_existing_json_files() {
    let pricing_paths = collect_fixture_paths_under("pricing")
        .expect("pricing fixture discovery should walk the pricing directory");
    let relatives = pricing_paths
        .iter()
        .map(|path| fixture_relative_path(path).expect("pricing fixture should be under data root"))
        .collect::<BTreeSet<_>>();

    assert!(relatives.contains("pricing/cds/cds_5y_par_spread.json"));
    assert!(relatives.contains("pricing/irs/usd_sofr_5y_receive_fixed_swpm.json"));
    assert!(!relatives.contains("pricing/cds/cds_5y_running_upfront.json"));
}

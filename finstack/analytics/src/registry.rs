//! Embedded analytics defaults registry.
//!
//! Python convenience defaults live in versioned JSON so annualization,
//! rolling-window, and tail-confidence defaults can be governed as data.

use std::sync::OnceLock;

use finstack_core::config::FinstackConfig;
use finstack_core::{Error, Result};
use serde::Deserialize;

/// Config extension key for overriding analytics defaults.
pub const ANALYTICS_DEFAULTS_EXTENSION_KEY: &str = "analytics.defaults.v1";

const ANALYTICS_DEFAULTS: &str = include_str!("../data/defaults/analytics_defaults.v1.json");
const EXPECTED_SCHEMA: &str = "finstack.analytics.defaults.v1";
const EXPECTED_VERSION: u32 = 1;

static EMBEDDED_DEFAULTS: OnceLock<Result<AnalyticsDefaults>> = OnceLock::new();

/// Resolved analytics defaults.
#[derive(Debug, Clone)]
pub struct AnalyticsDefaults {
    /// Python binding convenience defaults.
    pub python_bindings: RiskMetricPythonDefaults,
}

/// Defaults used by Python risk-metric bindings.
#[derive(Debug, Clone)]
pub struct RiskMetricPythonDefaults {
    /// Mean-return defaults.
    pub mean_return: MeanReturnDefaults,
    /// Volatility defaults.
    pub volatility: AnnualizedMetricDefaults,
    /// Downside-deviation defaults.
    pub downside_deviation: DownsideDeviationDefaults,
    /// Sortino-ratio defaults.
    pub sortino: SortinoDefaults,
    /// Modified-Sharpe defaults.
    pub modified_sharpe: ModifiedSharpeDefaults,
    /// Rolling risk-metric defaults.
    pub rolling: RollingDefaults,
    /// Tail-risk defaults.
    pub tail_risk: TailRiskDefaults,
    /// Benchmark-relative analytics defaults.
    pub benchmark: BenchmarkDefaults,
    /// Monte Carlo ruin-estimation defaults.
    pub ruin_model: RuinModelDefaults,
    /// Lookback-period defaults.
    pub lookback: LookbackDefaults,
}

/// Defaults for mean-return calculations.
#[derive(Debug, Clone, Deserialize)]
pub struct MeanReturnDefaults {
    /// Whether to annualize the mean return.
    pub annualize: bool,
    /// Annualization factor.
    pub ann_factor: f64,
}

/// Defaults for metrics with annualization controls.
#[derive(Debug, Clone, Deserialize)]
pub struct AnnualizedMetricDefaults {
    /// Whether to annualize the metric.
    pub annualize: bool,
    /// Annualization factor.
    pub ann_factor: f64,
}

/// Defaults for downside-deviation calculations.
#[derive(Debug, Clone, Deserialize)]
pub struct DownsideDeviationDefaults {
    /// Minimum acceptable return.
    pub mar: f64,
    /// Whether to annualize downside deviation.
    pub annualize: bool,
    /// Annualization factor.
    pub ann_factor: f64,
}

/// Defaults for Sortino-ratio calculations.
#[derive(Debug, Clone, Deserialize)]
pub struct SortinoDefaults {
    /// Whether to annualize the return and downside deviation.
    pub annualize: bool,
    /// Annualization factor.
    pub ann_factor: f64,
    /// Minimum acceptable return.
    pub mar: f64,
}

/// Defaults for modified-Sharpe calculations.
#[derive(Debug, Clone, Deserialize)]
pub struct ModifiedSharpeDefaults {
    /// Risk-free rate.
    pub risk_free_rate: f64,
    /// VaR confidence level.
    pub confidence: f64,
    /// Annualization factor.
    pub ann_factor: f64,
}

/// Defaults for rolling risk metrics.
#[derive(Debug, Clone, Deserialize)]
pub struct RollingDefaults {
    /// Rolling window length.
    pub window: usize,
    /// Annualization factor.
    pub ann_factor: f64,
    /// Risk-free rate for rolling Sharpe.
    pub risk_free_rate: f64,
}

/// Defaults for tail-risk metrics.
#[derive(Debug, Clone, Deserialize)]
pub struct TailRiskDefaults {
    /// Tail confidence level.
    pub confidence: f64,
}

/// Defaults for benchmark-relative analytics.
#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkDefaults {
    /// Annualization factor.
    pub ann_factor: f64,
    /// Rolling greeks window length.
    pub rolling_window: usize,
    /// Whether benchmark metrics annualize by default.
    pub annualize: bool,
}

/// Defaults for Monte Carlo ruin estimation.
#[derive(Debug, Clone, Deserialize)]
pub struct RuinModelDefaults {
    /// Number of forward periods to simulate.
    pub horizon_periods: usize,
    /// Number of bootstrap paths.
    pub n_paths: usize,
    /// Bootstrap block size.
    pub block_size: usize,
    /// RNG seed.
    pub seed: u64,
    /// Confidence level for intervals.
    pub confidence_level: f64,
}

/// Defaults for lookback-period analytics.
#[derive(Debug, Clone)]
pub struct LookbackDefaults {
    /// Default fiscal calendar.
    pub default_fiscal_calendar: FiscalCalendarDefaults,
}

/// Fiscal calendar default metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct FiscalCalendarDefaults {
    /// Stable calendar identifier.
    pub id: String,
    /// Holiday calendar used to align fiscal start dates.
    pub calendar_id: String,
    /// Fiscal year start month.
    pub start_month: u8,
    /// Fiscal year start day.
    pub start_day: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefaultsFile {
    schema: Option<String>,
    version: Option<u32>,
    python_bindings: RiskMetricPythonDefaultsFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RiskMetricPythonDefaultsFile {
    mean_return: MeanReturnDefaults,
    volatility: AnnualizedMetricDefaults,
    downside_deviation: DownsideDeviationDefaults,
    sortino: SortinoDefaults,
    modified_sharpe: ModifiedSharpeDefaults,
    rolling: RollingDefaults,
    tail_risk: TailRiskDefaults,
    benchmark: BenchmarkDefaults,
    ruin_model: RuinModelDefaults,
    lookback: LookbackDefaultsFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LookbackDefaultsFile {
    default_fiscal_calendar: FiscalCalendarDefaults,
}

/// Return the embedded analytics defaults.
pub fn embedded_defaults() -> Result<&'static AnalyticsDefaults> {
    match EMBEDDED_DEFAULTS.get_or_init(parse_embedded_defaults) {
        Ok(defaults) => Ok(defaults),
        Err(err) => Err(err.clone()),
    }
}

/// Loads analytics defaults from configuration or falls back to embedded defaults.
pub fn defaults_from_config(config: &FinstackConfig) -> Result<AnalyticsDefaults> {
    if let Some(value) = config.extensions.get(ANALYTICS_DEFAULTS_EXTENSION_KEY) {
        let file: DefaultsFile = serde_json::from_value(value.clone()).map_err(|err| {
            tracing::warn!(
                extension_key = ANALYTICS_DEFAULTS_EXTENSION_KEY,
                error = %err,
                "failed to parse analytics defaults extension"
            );
            Error::Validation(format!(
                "failed to parse analytics defaults extension: {err}"
            ))
        })?;
        defaults_from_file(file)
    } else {
        tracing::debug!(
            extension_key = ANALYTICS_DEFAULTS_EXTENSION_KEY,
            "analytics defaults extension missing; using embedded defaults"
        );
        Ok(embedded_defaults()?.clone())
    }
}

fn parse_embedded_defaults() -> Result<AnalyticsDefaults> {
    let file: DefaultsFile = serde_json::from_str(ANALYTICS_DEFAULTS).map_err(|err| {
        tracing::warn!(
            error = %err,
            "failed to parse embedded analytics defaults"
        );
        Error::Validation(format!(
            "failed to parse embedded analytics defaults: {err}"
        ))
    })?;
    defaults_from_file(file)
}

fn defaults_from_file(file: DefaultsFile) -> Result<AnalyticsDefaults> {
    validate_file(&file)?;
    Ok(AnalyticsDefaults {
        python_bindings: RiskMetricPythonDefaults {
            mean_return: file.python_bindings.mean_return,
            volatility: file.python_bindings.volatility,
            downside_deviation: file.python_bindings.downside_deviation,
            sortino: file.python_bindings.sortino,
            modified_sharpe: file.python_bindings.modified_sharpe,
            rolling: file.python_bindings.rolling,
            tail_risk: file.python_bindings.tail_risk,
            benchmark: file.python_bindings.benchmark,
            ruin_model: file.python_bindings.ruin_model,
            lookback: LookbackDefaults {
                default_fiscal_calendar: file.python_bindings.lookback.default_fiscal_calendar,
            },
        },
    })
}

fn validate_file(file: &DefaultsFile) -> Result<()> {
    validate_schema_version(file)?;
    validate_field(
        "python_bindings.mean_return.ann_factor",
        file.python_bindings.mean_return.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.volatility.ann_factor",
        file.python_bindings.volatility.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.downside_deviation.mar",
        file.python_bindings.downside_deviation.mar,
        f64::is_finite,
        "must be finite",
    )?;
    validate_field(
        "python_bindings.downside_deviation.ann_factor",
        file.python_bindings.downside_deviation.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.sortino.mar",
        file.python_bindings.sortino.mar,
        f64::is_finite,
        "must be finite",
    )?;
    validate_field(
        "python_bindings.sortino.ann_factor",
        file.python_bindings.sortino.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.modified_sharpe.risk_free_rate",
        file.python_bindings.modified_sharpe.risk_free_rate,
        f64::is_finite,
        "must be finite",
    )?;
    validate_field(
        "python_bindings.modified_sharpe.confidence",
        file.python_bindings.modified_sharpe.confidence,
        |value| value.is_finite() && (0.0..1.0).contains(&value),
        "must be finite and between 0 and 1",
    )?;
    validate_field(
        "python_bindings.modified_sharpe.ann_factor",
        file.python_bindings.modified_sharpe.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.rolling.window",
        file.python_bindings.rolling.window,
        |value| value > 0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.rolling.ann_factor",
        file.python_bindings.rolling.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.rolling.risk_free_rate",
        file.python_bindings.rolling.risk_free_rate,
        f64::is_finite,
        "must be finite",
    )?;
    validate_field(
        "python_bindings.tail_risk.confidence",
        file.python_bindings.tail_risk.confidence,
        |value| value.is_finite() && (0.0..1.0).contains(&value),
        "must be finite and between 0 and 1",
    )?;
    validate_field(
        "python_bindings.benchmark.ann_factor",
        file.python_bindings.benchmark.ann_factor,
        |value| value.is_finite() && value > 0.0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.benchmark.rolling_window",
        file.python_bindings.benchmark.rolling_window,
        |value| value > 0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.ruin_model.horizon_periods",
        file.python_bindings.ruin_model.horizon_periods,
        |value| value > 0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.ruin_model.n_paths",
        file.python_bindings.ruin_model.n_paths,
        |value| value > 0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.ruin_model.block_size",
        file.python_bindings.ruin_model.block_size,
        |value| value > 0,
        "must be positive",
    )?;
    validate_field(
        "python_bindings.ruin_model.confidence_level",
        file.python_bindings.ruin_model.confidence_level,
        |value| value.is_finite() && (0.0..1.0).contains(&value),
        "must be finite and between 0 and 1",
    )?;
    validate_fiscal_calendar(
        "python_bindings.lookback.default_fiscal_calendar",
        &file.python_bindings.lookback.default_fiscal_calendar,
    )
}

fn validate_schema_version(file: &DefaultsFile) -> Result<()> {
    match file.schema.as_deref() {
        Some(EXPECTED_SCHEMA) => {}
        Some(schema) => {
            tracing::warn!(
                expected_schema = EXPECTED_SCHEMA,
                actual_schema = schema,
                "analytics defaults schema mismatch"
            );
            return Err(Error::Validation(format!(
                "analytics defaults schema must be {EXPECTED_SCHEMA}, got {schema}"
            )));
        }
        None => {
            tracing::warn!("analytics defaults schema missing");
            return Err(Error::Validation(
                "analytics defaults schema is required".to_string(),
            ));
        }
    }

    match file.version {
        Some(EXPECTED_VERSION) => Ok(()),
        Some(version) => {
            tracing::warn!(
                expected_version = EXPECTED_VERSION,
                actual_version = version,
                "analytics defaults version mismatch"
            );
            Err(Error::Validation(format!(
                "analytics defaults version must be {EXPECTED_VERSION}, got {version}"
            )))
        }
        None => {
            tracing::warn!("analytics defaults version missing");
            Err(Error::Validation(
                "analytics defaults version is required".to_string(),
            ))
        }
    }
}

fn validate_field<T>(
    label: &str,
    value: T,
    is_valid: impl FnOnce(T) -> bool,
    requirement: &str,
) -> Result<()> {
    if !is_valid(value) {
        tracing::warn!(
            field = label,
            requirement,
            "analytics defaults validation failed"
        );
        return Err(Error::Validation(format!("{label} {requirement}")));
    }
    Ok(())
}

fn validate_fiscal_calendar(label: &str, calendar: &FiscalCalendarDefaults) -> Result<()> {
    if calendar.id.trim().is_empty() {
        return Err(Error::Validation(format!("{label}.id must not be blank")));
    }
    if calendar.calendar_id.trim().is_empty() {
        return Err(Error::Validation(format!(
            "{label}.calendar_id must not be blank"
        )));
    }
    if finstack_core::dates::CalendarRegistry::global()
        .resolve_str(&calendar.calendar_id)
        .is_none()
    {
        return Err(Error::calendar_not_found_with_suggestions(
            calendar.calendar_id.clone(),
            finstack_core::dates::available_calendars(),
        ));
    }
    finstack_core::dates::FiscalConfig::new(calendar.start_month, calendar.start_day)
        .map(|_| ())
        .map_err(|err| {
            Error::Validation(format!("{label} has invalid fiscal start month/day: {err}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults_value() -> serde_json::Value {
        serde_json::from_str(ANALYTICS_DEFAULTS).expect("embedded defaults JSON should parse")
    }

    fn parse_file(value: serde_json::Value) -> DefaultsFile {
        serde_json::from_value(value).expect("defaults file should deserialize")
    }

    fn validation_message(result: Result<()>) -> String {
        match result.expect_err("validation should fail") {
            Error::Validation(message) => message,
            err => panic!("expected validation error, got {err:?}"),
        }
    }

    #[test]
    fn validate_file_requires_schema() {
        let mut value = defaults_value();
        value.as_object_mut().expect("object").remove("schema");

        let message = validation_message(validate_file(&parse_file(value)));

        assert!(message.contains("schema"));
    }

    #[test]
    fn validate_file_rejects_wrong_schema() {
        let mut value = defaults_value();
        value["schema"] = serde_json::Value::String("wrong.schema".to_string());

        let message = validation_message(validate_file(&parse_file(value)));

        assert!(message.contains("schema"));
    }

    #[test]
    fn validate_file_requires_version() {
        let mut value = defaults_value();
        value.as_object_mut().expect("object").remove("version");

        let message = validation_message(validate_file(&parse_file(value)));

        assert!(message.contains("version"));
    }

    #[test]
    fn validate_file_rejects_wrong_version() {
        let mut value = defaults_value();
        value["version"] = serde_json::Value::from(2);

        let message = validation_message(validate_file(&parse_file(value)));

        assert!(message.contains("version"));
    }

    #[test]
    fn validate_file_rejects_invalid_numeric_field() {
        let mut value = defaults_value();
        value["python_bindings"]["rolling"]["window"] = serde_json::Value::from(0);

        let message = validation_message(validate_file(&parse_file(value)));

        assert!(message.contains("python_bindings.rolling.window"));
    }
}

//! Embedded calibration defaults.

use std::sync::OnceLock;

use finstack_core::config::FinstackConfig;
use finstack_core::{Error, Result};
use serde::Deserialize;

/// Config extension key for overriding calibration defaults.
pub const CALIBRATION_DEFAULTS_EXTENSION_KEY: &str = "valuations.calibration_defaults.v1";

const CALIBRATION_DEFAULTS: &str =
    include_str!("../../data/calibration/calibration_defaults.v1.json");

static EMBEDDED_DEFAULTS: OnceLock<Result<CalibrationDefaults>> = OnceLock::new();

/// Resolved calibration defaults.
#[derive(Debug, Clone)]
pub struct CalibrationDefaults {
    /// Validation defaults.
    pub validation: CalibrationValidationDefaults,
    /// LMM calibration defaults.
    pub lmm_calibration: LmmCalibrationDefaults,
}

/// Defaults used by calibration preflight validation.
#[derive(Debug, Clone, Deserialize)]
pub struct CalibrationValidationDefaults {
    /// Absolute tolerance used when comparing configured and quoted recovery rates.
    pub recovery_rate_abs_tolerance: f64,
    /// Minimum LGD denominator used for hazard-rate initial guesses.
    pub minimum_lgd_for_hazard_guess: f64,
}

/// Defaults used by LMM calibration.
#[derive(Debug, Clone, Deserialize)]
pub struct LmmCalibrationDefaults {
    /// Number of Brownian factors.
    pub num_factors: usize,
    /// Initial correlation decay parameter.
    pub beta_init: f64,
    /// Whether to optimise beta.
    pub calibrate_beta: bool,
    /// Optimizer tolerance.
    pub tolerance: f64,
    /// Maximum optimizer iterations.
    pub max_iterations: usize,
    /// Whether strict diagnostics are enabled.
    pub strict_mode: bool,
    /// Maximum allowed PCA variance loss.
    pub pca_variance_loss_tolerance: f64,
    /// Optional MC validation defaults enabled by default.
    pub mc_validation: Option<LmmMcValidationDefaults>,
    /// Standard MC validation defaults for explicit default construction.
    pub mc_validation_defaults: LmmMcValidationDefaults,
}

/// Defaults used by LMM MC validation.
#[derive(Debug, Clone, Deserialize)]
pub struct LmmMcValidationDefaults {
    /// Number of Monte Carlo paths.
    pub num_paths: usize,
    /// Predictor-corrector grid steps per year.
    pub num_steps_per_year: usize,
    /// RNG seed.
    pub seed: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefaultsFile {
    schema: Option<String>,
    version: Option<u32>,
    validation: CalibrationValidationDefaults,
    lmm_calibration: LmmCalibrationDefaults,
}

/// Return the embedded calibration defaults.
pub fn embedded_defaults() -> Result<&'static CalibrationDefaults> {
    match EMBEDDED_DEFAULTS.get_or_init(parse_embedded_defaults) {
        Ok(defaults) => Ok(defaults),
        Err(err) => Err(err.clone()),
    }
}

/// Panic-on-failure access for infallible calibration paths backed by embedded data.
#[must_use]
#[allow(clippy::expect_used)]
pub fn embedded_defaults_or_panic() -> &'static CalibrationDefaults {
    embedded_defaults().expect("embedded calibration defaults are compile-time assets")
}

/// Loads calibration defaults from configuration or falls back to embedded defaults.
pub fn defaults_from_config(config: &FinstackConfig) -> Result<CalibrationDefaults> {
    if let Some(value) = config.extensions.get(CALIBRATION_DEFAULTS_EXTENSION_KEY) {
        let file: DefaultsFile = serde_json::from_value(value.clone()).map_err(|err| {
            Error::Validation(format!(
                "failed to parse calibration defaults extension: {err}"
            ))
        })?;
        defaults_from_file(file)
    } else {
        Ok(embedded_defaults()?.clone())
    }
}

fn parse_embedded_defaults() -> Result<CalibrationDefaults> {
    let file: DefaultsFile = serde_json::from_str(CALIBRATION_DEFAULTS).map_err(|err| {
        Error::Validation(format!(
            "failed to parse embedded calibration defaults: {err}"
        ))
    })?;
    defaults_from_file(file)
}

fn defaults_from_file(file: DefaultsFile) -> Result<CalibrationDefaults> {
    validate_file(&file)?;
    Ok(CalibrationDefaults {
        validation: file.validation,
        lmm_calibration: file.lmm_calibration,
    })
}

fn validate_file(file: &DefaultsFile) -> Result<()> {
    let _schema = &file.schema;
    let _version = file.version;
    validate_nonnegative_finite(
        "validation.recovery_rate_abs_tolerance",
        file.validation.recovery_rate_abs_tolerance,
    )?;
    validate_positive_finite(
        "validation.minimum_lgd_for_hazard_guess",
        file.validation.minimum_lgd_for_hazard_guess,
    )?;
    validate_lmm_calibration(&file.lmm_calibration)
}

fn validate_lmm_calibration(defaults: &LmmCalibrationDefaults) -> Result<()> {
    validate_positive_usize("lmm_calibration.num_factors", defaults.num_factors)?;
    validate_positive_finite("lmm_calibration.beta_init", defaults.beta_init)?;
    validate_positive_finite("lmm_calibration.tolerance", defaults.tolerance)?;
    validate_positive_usize("lmm_calibration.max_iterations", defaults.max_iterations)?;
    validate_nonnegative_finite(
        "lmm_calibration.pca_variance_loss_tolerance",
        defaults.pca_variance_loss_tolerance,
    )?;
    if let Some(mc) = &defaults.mc_validation {
        validate_lmm_mc_validation("lmm_calibration.mc_validation", mc)?;
    }
    validate_lmm_mc_validation(
        "lmm_calibration.mc_validation_defaults",
        &defaults.mc_validation_defaults,
    )?;
    let _calibrate_beta = defaults.calibrate_beta;
    let _strict_mode = defaults.strict_mode;
    Ok(())
}

fn validate_lmm_mc_validation(label: &str, defaults: &LmmMcValidationDefaults) -> Result<()> {
    validate_positive_usize(&format!("{label}.num_paths"), defaults.num_paths)?;
    validate_positive_usize(
        &format!("{label}.num_steps_per_year"),
        defaults.num_steps_per_year,
    )?;
    let _seed = defaults.seed;
    Ok(())
}

fn validate_nonnegative_finite(label: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(Error::Validation(format!(
            "{label} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn validate_positive_finite(label: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(Error::Validation(format!(
            "{label} must be finite and positive"
        )));
    }
    Ok(())
}

fn validate_positive_usize(label: &str, value: usize) -> Result<()> {
    if value == 0 {
        return Err(Error::Validation(format!("{label} must be positive")));
    }
    Ok(())
}

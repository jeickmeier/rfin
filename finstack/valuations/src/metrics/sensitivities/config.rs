//! Sensitivities configuration (user-facing defaults) resolved from `FinstackConfig` extensions.
//!
//! This module defines the versioned extension section:
//! - `valuations.sensitivities.v1`
//!
//! It is intended for settings that users commonly tweak and need to persist for reproducible
//! pipelines (Tier-3 configuration in the Rust code standards).

use finstack_core::config::FinstackConfig;

/// Extension section key for sensitivities defaults.
pub const SENSITIVITIES_CONFIG_KEY_V1: &str = "valuations.sensitivities.v1";

/// Standard risk bucket grid in years used for IR DV01 and credit CS01.
pub const STANDARD_BUCKETS_YEARS: [f64; 11] = [
    0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0,
];

/// Resolved (fully-populated) sensitivities configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct SensitivitiesConfig {
    /// Interest rate bump size in basis points (e.g., 1.0 = 1bp).
    pub rate_bump_bp: f64,
    /// Credit spread bump size in basis points (e.g., 1.0 = 1bp).
    pub credit_spread_bump_bp: f64,
    /// Spot bump size as a percentage (e.g., 0.01 = 1%).
    pub spot_bump_pct: f64,
    /// Vol bump size (absolute) as a percentage (e.g., 0.01 = 1% vol).
    pub vol_bump_pct: f64,
    /// Default DV01 key-rate buckets in years.
    pub dv01_buckets_years: Vec<f64>,
    /// Default CS01 key-rate buckets in years.
    pub cs01_buckets_years: Vec<f64>,
}

impl Default for SensitivitiesConfig {
    fn default() -> Self {
        Self {
            rate_bump_bp: 1.0,
            credit_spread_bump_bp: 1.0,
            spot_bump_pct: crate::metrics::bump_sizes::SPOT,
            vol_bump_pct: crate::metrics::bump_sizes::VOLATILITY,
            dv01_buckets_years: STANDARD_BUCKETS_YEARS.to_vec(),
            cs01_buckets_years: STANDARD_BUCKETS_YEARS.to_vec(),
        }
    }
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivitiesConfigV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_bump_bp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_spread_bump_bp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot_bump_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_bump_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dv01_buckets_years: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cs01_buckets_years: Option<Vec<f64>>,
}

fn ensure_finite_positive(name: &str, v: f64) -> finstack_core::Result<()> {
    if !v.is_finite() || v <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "Invalid sensitivities config: '{name}' must be finite and > 0, got {v}"
        )));
    }
    Ok(())
}

fn ensure_bucket_grid(name: &str, buckets: &[f64]) -> finstack_core::Result<()> {
    if buckets.is_empty() {
        return Err(finstack_core::Error::Validation(format!(
            "Invalid sensitivities config: '{name}' must be non-empty"
        )));
    }
    for (i, &t) in buckets.iter().enumerate() {
        if !t.is_finite() || t <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Invalid sensitivities config: '{name}[{i}]' must be finite and > 0, got {t}"
            )));
        }
        if i > 0 && t <= buckets[i - 1] {
            return Err(finstack_core::Error::Validation(format!(
                "Invalid sensitivities config: '{name}' must be strictly increasing (got {prev} then {t})",
                prev = buckets[i - 1]
            )));
        }
    }
    Ok(())
}

/// Resolve sensitivities defaults from a `FinstackConfig`.
///
/// If the extension section `valuations.sensitivities.v1` is present, its fields override
/// the defaults; otherwise defaults are used.
#[cfg(feature = "serde")]
pub fn from_finstack_config_or_default(cfg: &FinstackConfig) -> finstack_core::Result<SensitivitiesConfig> {
    let mut base = SensitivitiesConfig::default();

    if let Some(raw) = cfg.extensions.get(SENSITIVITIES_CONFIG_KEY_V1) {
        let overrides: SensitivitiesConfigV1 =
            serde_json::from_value(raw.clone()).map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Failed to parse extension '{}': {}",
                    SENSITIVITIES_CONFIG_KEY_V1, e
                ),
                category: "config".to_string(),
            })?;

        if let Some(v) = overrides.rate_bump_bp {
            ensure_finite_positive("rate_bump_bp", v)?;
            base.rate_bump_bp = v;
        }
        if let Some(v) = overrides.credit_spread_bump_bp {
            ensure_finite_positive("credit_spread_bump_bp", v)?;
            base.credit_spread_bump_bp = v;
        }
        if let Some(v) = overrides.spot_bump_pct {
            ensure_finite_positive("spot_bump_pct", v)?;
            base.spot_bump_pct = v;
        }
        if let Some(v) = overrides.vol_bump_pct {
            ensure_finite_positive("vol_bump_pct", v)?;
            base.vol_bump_pct = v;
        }
        if let Some(v) = overrides.dv01_buckets_years {
            ensure_bucket_grid("dv01_buckets_years", &v)?;
            base.dv01_buckets_years = v;
        }
        if let Some(v) = overrides.cs01_buckets_years {
            ensure_bucket_grid("cs01_buckets_years", &v)?;
            base.cs01_buckets_years = v;
        }
    }

    Ok(base)
}

/// Resolve sensitivities defaults without requiring the `serde` feature.
#[cfg(not(feature = "serde"))]
pub fn from_finstack_config_or_default(_cfg: &FinstackConfig) -> finstack_core::Result<SensitivitiesConfig> {
    Ok(SensitivitiesConfig::default())
}



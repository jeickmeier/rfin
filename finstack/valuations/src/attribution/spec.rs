//! JSON specification and execution framework for attribution.
//!
//! Provides serializable specs for defining complete attribution runs in JSON,
//! with stable schemas and deterministic round-trip serialization.

use super::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_waterfall,
    types::JsonEnvelope, AttributionMethod, PnlAttribution,
};
use crate::instruments::json_loader::InstrumentJson;
use crate::metrics::MetricId;
use finstack_core::{
    config::{FinstackConfig, ResultsMeta},
    dates::Date,
    market_data::context::{MarketContext, MarketContextState},
    Result,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Schema version for attribution serialization.
pub const ATTRIBUTION_SCHEMA_V1: &str = "finstack.attribution/1";

/// Top-level envelope for attribution specifications.
///
/// Mirrors the calibration and instrument envelope patterns with schema versioning
/// and strict field validation for long-term JSON stability.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionEnvelope {
    /// Schema version identifier (currently "finstack.attribution/1")
    pub schema: String,
    /// The attribution specification
    pub attribution: AttributionSpec,
}

impl AttributionEnvelope {
    /// Create a new attribution envelope with the current schema version.
    pub fn new(attribution: AttributionSpec) -> Self {
        Self {
            schema: ATTRIBUTION_SCHEMA_V1.to_string(),
            attribution,
        }
    }

    /// Execute the attribution and return the result envelope.
    pub fn execute(&self) -> Result<AttributionResultEnvelope> {
        let result = self.attribution.execute()?;
        Ok(AttributionResultEnvelope::new(result))
    }
}

impl JsonEnvelope for AttributionEnvelope {
    fn parse_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to parse attribution JSON: {}", e),
            category: "json_parse".to_string(),
        }
    }

    fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to serialize attribution: {}", e),
            category: "json_serialize".to_string(),
        }
    }
}

/// Attribution specification for a single P&L attribution run.
///
/// Contains all data needed to perform attribution: instrument, market snapshots,
/// dates, and methodology.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionSpec {
    /// Instrument to attribute (as JSON envelope)
    pub instrument: InstrumentJson,
    /// Market context at T₀
    pub market_t0: MarketContextState,
    /// Market context at T₁
    pub market_t1: MarketContextState,
    /// Valuation date at T₀
    pub as_of_t0: Date,
    /// Valuation date at T₁
    pub as_of_t1: Date,
    /// Attribution methodology
    pub method: AttributionMethod,
    /// Optional model parameters at T₀ (for attributing parameter changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_params_t0: Option<crate::attribution::model_params::ModelParamsSnapshot>,
    /// Optional configuration overrides (defaults to FinstackConfig::default())
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AttributionConfig>,
}

/// Optional configuration for attribution runs.
///
/// Allows overriding default tolerances and metrics for attribution calculations.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionConfig {
    /// Absolute tolerance for residual validation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_abs: Option<f64>,
    /// Percentage tolerance for residual validation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_pct: Option<f64>,
    /// Metrics to compute for metrics-based attribution (optional)
    /// If not provided, a default set will be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    /// Strict validation mode (if true, errors during attribution will propagate instead of being logged)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_validation: Option<bool>,
}

impl AttributionSpec {
    /// Execute the attribution specification.
    ///
    /// Returns a complete result with the P&L attribution and metadata.
    pub fn execute(&self) -> Result<AttributionResult> {
        // Reconstruct instrument from JSON
        let instrument = self.instrument.clone().into_boxed()?;
        let instrument_arc = std::sync::Arc::from(instrument);

        // Reconstruct market contexts
        let market_t0 = MarketContext::try_from(self.market_t0.clone())?;
        let market_t1 = MarketContext::try_from(self.market_t1.clone())?;

        // Get config (use defaults if not provided)
        let config = FinstackConfig::default();

        // Determine strict validation
        let strict_validation = self
            .config
            .as_ref()
            .and_then(|c| c.strict_validation)
            .unwrap_or(false);

        // Execute attribution based on method
        let mut attribution = match &self.method {
            AttributionMethod::Parallel => attribute_pnl_parallel(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                self.model_params_t0.as_ref(),
            )?,

            AttributionMethod::Waterfall(order) => attribute_pnl_waterfall(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                order.clone(),
                strict_validation,
                self.model_params_t0.as_ref(),
            )?,

            AttributionMethod::MetricsBased => {
                // Determine metrics to use
                let metrics = if let Some(ref cfg) = self.config {
                    if let Some(ref metric_names) = cfg.metrics {
                        metric_names
                            .iter()
                            .filter_map(|name| MetricId::from_str(name).ok())
                            .collect()
                    } else {
                        default_attribution_metrics()
                    }
                } else {
                    default_attribution_metrics()
                };

                // Compute valuations with metrics
                let val_t0 =
                    instrument_arc.price_with_metrics(&market_t0, self.as_of_t0, &metrics)?;
                let val_t1 =
                    instrument_arc.price_with_metrics(&market_t1, self.as_of_t1, &metrics)?;

                attribute_pnl_metrics_based(
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    &val_t0,
                    &val_t1,
                    self.as_of_t0,
                    self.as_of_t1,
                )?
            }
        };

        // Apply tolerance overrides if provided
        if let Some(ref cfg) = self.config {
            if let Some(tol_abs) = cfg.tolerance_abs {
                attribution.meta.tolerance_abs = tol_abs;
            }
            if let Some(tol_pct) = cfg.tolerance_pct {
                attribution.meta.tolerance_pct = tol_pct;
            }
        }

        // Create results metadata
        let results_meta = finstack_core::config::results_meta(&config);

        Ok(AttributionResult {
            attribution,
            results_meta,
        })
    }
}

/// Default set of metrics for metrics-based attribution.
pub fn default_attribution_metrics() -> Vec<MetricId> {
    vec![
        // First-order metrics
        MetricId::Theta,       // Time decay (carry)
        MetricId::Dv01,        // Interest rate sensitivity
        MetricId::Cs01,        // Credit spread sensitivity
        MetricId::Vega,        // Volatility sensitivity
        MetricId::Delta,       // Delta for options/equity
        MetricId::Fx01,        // FX sensitivity
        MetricId::Inflation01, // Inflation sensitivity
        MetricId::Dividend01,  // Dividend sensitivity
        // Second-order metrics
        MetricId::Gamma,              // Spot convexity
        MetricId::Convexity,          // Rate convexity (bonds)
        MetricId::IrConvexity,        // Rate convexity (swaps)
        MetricId::Volga,              // Vol convexity
        MetricId::Vanna,              // Cross-gamma (spot-vol)
        MetricId::CsGamma,            // Credit spread convexity
        MetricId::InflationConvexity, // Inflation convexity
    ]
}

/// Complete attribution result with P&L attribution and metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionResult {
    /// P&L attribution with factor decomposition
    pub attribution: PnlAttribution,
    /// Results metadata (timestamp, version, rounding context, etc.)
    pub results_meta: ResultsMeta,
}

/// Top-level envelope for attribution results.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionResultEnvelope {
    /// Schema version identifier
    pub schema: String,
    /// The attribution result
    pub result: AttributionResult,
}

impl AttributionResultEnvelope {
    /// Create a new result envelope with the current schema version.
    pub fn new(result: AttributionResult) -> Self {
        Self {
            schema: ATTRIBUTION_SCHEMA_V1.to_string(),
            result,
        }
    }
}

impl JsonEnvelope for AttributionResultEnvelope {
    fn parse_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to parse attribution result JSON: {}", e),
            category: "json_parse".to_string(),
        }
    }

    fn serialize_error(e: serde_json::Error) -> finstack_core::Error {
        finstack_core::Error::Calibration {
            message: format!("Failed to serialize attribution result: {}", e),
            category: "json_serialize".to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::create_date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    #[allow(clippy::unwrap_used)] // Test code
    fn test_attribution_envelope_roundtrip() {
        use crate::instruments::Bond;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            create_date(2024, Month::January, 1).expect("Valid test date"),
            create_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .unwrap();

        let spec = AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: MarketContextState {
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
            },
            market_t1: MarketContextState {
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
            },
            as_of_t0: create_date(2025, Month::January, 1).expect("Valid test date"),
            as_of_t1: create_date(2025, Month::January, 2).expect("Valid test date"),
            method: AttributionMethod::Parallel,
            model_params_t0: None,
            config: None,
        };

        let envelope = AttributionEnvelope::new(spec);
        let json = serde_json::to_string_pretty(&envelope)
            .expect("JSON serialization should succeed in test");
        let parsed: AttributionEnvelope =
            serde_json::from_str(&json).expect("JSON deserialization should succeed in test");

        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(parsed.attribution.as_of_t0, envelope.attribution.as_of_t0);
        assert_eq!(parsed.attribution.as_of_t1, envelope.attribution.as_of_t1);
    }

    #[test]
    fn test_attribution_config_optional_fields() {
        let config = AttributionConfig {
            tolerance_abs: Some(0.01),
            tolerance_pct: Some(0.001),
            metrics: None,
            strict_validation: Some(true),
        };

        let json =
            serde_json::to_value(&config).expect("JSON value conversion should succeed in test");
        assert!(json.get("tolerance_abs").is_some());
        assert!(json.get("tolerance_pct").is_some());
        assert!(json.get("strict_validation").is_some());
        // metrics should not be present when None
        assert!(json.get("metrics").is_none());
    }

    #[test]
    fn test_attribution_envelope_json_envelope_trait() {
        use crate::instruments::Bond;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            create_date(2024, Month::January, 1).expect("Valid test date"),
            create_date(2034, Month::January, 1).expect("Valid test date"),
            "USD-OIS",
        )
        .unwrap();

        let spec = AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: MarketContextState {
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
            },
            market_t1: MarketContextState {
                curves: vec![],
                fx: None,
                surfaces: vec![],
                prices: std::collections::BTreeMap::new(),
                series: vec![],
                inflation_indices: vec![],
                dividends: vec![],
                credit_indices: vec![],
                collateral: std::collections::BTreeMap::new(),
            },
            as_of_t0: create_date(2025, Month::January, 1).expect("Valid test date"),
            as_of_t1: create_date(2025, Month::January, 2).expect("Valid test date"),
            method: AttributionMethod::Parallel,
            model_params_t0: None,
            config: None,
        };

        let envelope = AttributionEnvelope::new(spec);

        // Test to_json from JsonEnvelope trait
        let json = envelope.to_json().expect("to_json should succeed");
        assert!(json.contains("finstack.attribution/1"));

        // Test from_json from JsonEnvelope trait
        let parsed = AttributionEnvelope::from_json(&json).expect("from_json should succeed");
        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(parsed.attribution.as_of_t0, envelope.attribution.as_of_t0);

        // Test from_reader from JsonEnvelope trait
        let reader = std::io::Cursor::new(json.as_bytes());
        let parsed_from_reader =
            AttributionEnvelope::from_reader(reader).expect("from_reader should succeed");
        assert_eq!(parsed_from_reader.schema, ATTRIBUTION_SCHEMA_V1);
    }

    #[test]
    fn test_attribution_result_envelope_json_envelope_trait() {
        use finstack_core::config::ResultsMeta;

        let total = Money::new(1000.0, Currency::USD);
        let attribution = PnlAttribution::new(
            total,
            "TEST-BOND",
            create_date(2025, Month::January, 1).expect("Valid test date"),
            create_date(2025, Month::January, 2).expect("Valid test date"),
            AttributionMethod::Parallel,
        );

        let result = AttributionResult {
            attribution,
            results_meta: ResultsMeta::default(),
        };

        let envelope = AttributionResultEnvelope::new(result);

        // Test to_json from JsonEnvelope trait
        let json = envelope.to_json().expect("to_json should succeed");
        assert!(json.contains("finstack.attribution/1"));

        // Test from_json from JsonEnvelope trait
        let parsed = AttributionResultEnvelope::from_json(&json).expect("from_json should succeed");
        assert_eq!(parsed.schema, ATTRIBUTION_SCHEMA_V1);
        assert_eq!(
            parsed.result.attribution.total_pnl,
            envelope.result.attribution.total_pnl
        );

        // Test from_reader from JsonEnvelope trait (newly available!)
        let reader = std::io::Cursor::new(json.as_bytes());
        let parsed_from_reader =
            AttributionResultEnvelope::from_reader(reader).expect("from_reader should succeed");
        assert_eq!(parsed_from_reader.schema, ATTRIBUTION_SCHEMA_V1);
    }
}

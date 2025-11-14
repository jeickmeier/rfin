//! JSON specification and execution framework for calibration.
//!
//! Provides serializable specs for defining complete calibration runs in JSON,
//! with stable schemas and deterministic round-trip serialization. Uses an explicit
//! pipeline approach where users define ordered calibration steps.

use super::{
    methods::{
        base_correlation::BaseCorrelationCalibrator, discount::DiscountCurveCalibrator,
        forward_curve::ForwardCurveCalibrator, hazard_curve::HazardCurveCalibrator,
        inflation_curve::InflationCurveCalibrator, sabr_surface::VolSurfaceCalibrator,
        swaption_vol::SwaptionVolCalibrator,
    },
    quote::{CreditQuote, InflationQuote, RatesQuote, VolQuote},
    CalibrationConfig, CalibrationReport, Calibrator,
};
use finstack_core::{
    config::ResultsMeta, currency::Currency, dates::Date, market_data::context::MarketContext,
    Result,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Schema version for calibration serialization.
pub const CALIBRATION_SCHEMA_V1: &str = "finstack.calibration/1";

/// Top-level envelope for calibration specifications.
///
/// Mirrors the instrument envelope pattern with schema versioning and
/// strict field validation for long-term JSON stability.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationEnvelope {
    /// Schema version identifier (currently "finstack.calibration/1")
    pub schema: String,
    /// The calibration specification
    pub calibration: CalibrationSpec,
}

impl CalibrationEnvelope {
    /// Create a new calibration envelope with the current schema version.
    pub fn new(calibration: CalibrationSpec) -> Self {
        Self {
            schema: CALIBRATION_SCHEMA_V1.to_string(),
            calibration,
        }
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| finstack_core::Error::Calibration {
            message: format!("Failed to parse calibration JSON: {}", e),
            category: "json_parse".to_string(),
        })
    }

    /// Parse from JSON reader.
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(|e| finstack_core::Error::Calibration {
            message: format!("Failed to parse calibration JSON: {}", e),
            category: "json_parse".to_string(),
        })
    }

    /// Execute the calibration and return the result envelope.
    pub fn execute(
        &self,
        initial_market: Option<MarketContext>,
    ) -> Result<CalibrationResultEnvelope> {
        let result = self.calibration.execute(initial_market)?;
        Ok(CalibrationResultEnvelope::new(result))
    }
}

/// Calibration specification using explicit pipeline mode.
///
/// Users define ordered calibration steps, each with its own calibrator
/// configuration and market quotes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationSpec {
    /// Base date for all calibrations
    pub base_date: Date,
    /// Base currency
    pub base_currency: Currency,
    /// Global calibration configuration (overridable per step)
    pub config: CalibrationConfig,
    /// Ordered calibration steps
    pub steps: Vec<CalibrationStep>,
    /// Schema version
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl CalibrationSpec {
    /// Execute the calibration specification.
    ///
    /// Returns a complete result with the final market context, merged report,
    /// and per-step diagnostics.
    pub fn execute(&self, initial_market: Option<MarketContext>) -> Result<CalibrationResult> {
        let mut context = initial_market.unwrap_or_default();
        let mut step_reports = BTreeMap::new();
        let mut all_residuals = BTreeMap::new();
        let mut total_iterations = 0;

        // Execute each step in order
        for (idx, step) in self.steps.iter().enumerate() {
            let step_key = format!("step_{:03}_{}", idx, step.step_name());
            let (updated_ctx, report) = step.execute(&context)?;
            context = updated_ctx;

            // Merge residuals
            for (key, value) in &report.residuals {
                all_residuals.insert(format!("{}_{}", step_key, key), *value);
            }
            total_iterations += report.iterations;

            step_reports.insert(step_key, report);
        }

        // Create merged report
        let merged_report =
            CalibrationReport::for_type("pipeline", all_residuals, total_iterations);

        // Create results metadata
        let results_meta =
            finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());

        Ok(CalibrationResult {
            final_market: (&context).into(),
            report: merged_report,
            step_reports,
            results_meta,
        })
    }
}

/// Individual calibration step in a pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CalibrationStep {
    /// Discount curve calibration step
    Discount {
        /// Calibrator configuration
        calibrator: DiscountCurveCalibrator,
        /// Rates quotes for this step
        quotes: Vec<RatesQuote>,
    },
    /// Forward curve calibration step
    Forward {
        /// Calibrator configuration
        calibrator: ForwardCurveCalibrator,
        /// Rates quotes for this step
        quotes: Vec<RatesQuote>,
    },
    /// Hazard curve calibration step
    Hazard {
        /// Calibrator configuration
        calibrator: HazardCurveCalibrator,
        /// Credit quotes for this step
        quotes: Vec<CreditQuote>,
    },
    /// Inflation curve calibration step
    Inflation {
        /// Calibrator configuration
        calibrator: InflationCurveCalibrator,
        /// Inflation quotes for this step
        quotes: Vec<InflationQuote>,
    },
    /// Volatility surface calibration step (equity/FX style)
    Vol {
        /// Calibrator configuration
        calibrator: VolSurfaceCalibrator,
        /// Vol quotes for this step
        quotes: Vec<VolQuote>,
    },
    /// Swaption volatility surface calibration step
    SwaptionVol {
        /// Calibrator configuration
        calibrator: SwaptionVolCalibrator,
        /// Vol quotes for this step
        quotes: Vec<VolQuote>,
    },
    /// Base correlation curve calibration step
    BaseCorrelation {
        /// Calibrator configuration
        calibrator: BaseCorrelationCalibrator,
        /// Credit quotes for this step
        quotes: Vec<CreditQuote>,
    },
}

impl CalibrationStep {
    /// Execute this calibration step.
    pub fn execute(&self, context: &MarketContext) -> Result<(MarketContext, CalibrationReport)> {
        match self {
            CalibrationStep::Discount { calibrator, quotes } => {
                let (curve, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_discount(curve), report))
            }
            CalibrationStep::Forward { calibrator, quotes } => {
                let (curve, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_forward(curve), report))
            }
            CalibrationStep::Hazard { calibrator, quotes } => {
                let (curve, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_hazard(curve), report))
            }
            CalibrationStep::Inflation { calibrator, quotes } => {
                let (curve, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_inflation(curve), report))
            }
            CalibrationStep::Vol { calibrator, quotes } => {
                let (surface, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_surface(surface), report))
            }
            CalibrationStep::SwaptionVol { calibrator, quotes } => {
                let (surface, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_surface(surface), report))
            }
            CalibrationStep::BaseCorrelation { calibrator, quotes } => {
                let (curve, report) = calibrator.calibrate(quotes, context)?;
                Ok((context.clone().insert_base_correlation(curve), report))
            }
        }
    }

    /// Get a human-readable name for this step.
    pub fn step_name(&self) -> &'static str {
        match self {
            CalibrationStep::Discount { .. } => "discount",
            CalibrationStep::Forward { .. } => "forward",
            CalibrationStep::Hazard { .. } => "hazard",
            CalibrationStep::Inflation { .. } => "inflation",
            CalibrationStep::Vol { .. } => "vol",
            CalibrationStep::SwaptionVol { .. } => "swaption_vol",
            CalibrationStep::BaseCorrelation { .. } => "base_correlation",
        }
    }
}

/// Complete calibration result with market snapshot and diagnostics.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationResult {
    /// Final calibrated market context (all curves, surfaces, prices, etc.)
    pub final_market: finstack_core::market_data::context::MarketContextState,
    /// Merged calibration report
    pub report: CalibrationReport,
    /// Per-step calibration reports (for pipeline mode)
    pub step_reports: BTreeMap<String, CalibrationReport>,
    /// Results metadata (timestamp, version, rounding context, etc.)
    pub results_meta: ResultsMeta,
}

/// Top-level envelope for calibration results.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationResultEnvelope {
    /// Schema version identifier
    pub schema: String,
    /// The calibration result
    pub result: CalibrationResult,
}

impl CalibrationResultEnvelope {
    /// Create a new result envelope with the current schema version.
    pub fn new(result: CalibrationResult) -> Self {
        Self {
            schema: CALIBRATION_SCHEMA_V1.to_string(),
            result,
        }
    }

    /// Serialize to JSON string.
    pub fn to_string(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| finstack_core::Error::Calibration {
            message: format!("Failed to serialize calibration result: {}", e),
            category: "json_serialize".to_string(),
        })
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| finstack_core::Error::Calibration {
            message: format!("Failed to parse calibration result JSON: {}", e),
            category: "json_parse".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::create_date;
    use time::Month;

    #[test]
    fn test_calibration_envelope_roundtrip() {
        let spec = CalibrationSpec {
            base_date: create_date(2025, Month::January, 1).unwrap(),
            base_currency: Currency::USD,
            config: CalibrationConfig::default(),
            steps: vec![],
            schema_version: 1,
        };

        let envelope = CalibrationEnvelope::new(spec);
        let json = serde_json::to_string_pretty(&envelope).unwrap();
        let parsed: CalibrationEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema, CALIBRATION_SCHEMA_V1);
    }

    #[test]
    fn test_pipeline_step_execution() {
        // This is a placeholder test that will be filled in with actual calibration
        // logic once we have the full infrastructure in place
        let _step = CalibrationStep::Discount {
            calibrator: DiscountCurveCalibrator::new(
                "USD-OIS",
                create_date(2025, Month::January, 1).unwrap(),
                Currency::USD,
            ),
            quotes: vec![],
        };

        // For now, just verify we can construct the step
        // Test will be expanded when full pipeline execution is tested
    }
}

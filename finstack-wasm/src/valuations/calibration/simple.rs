//! Simple calibration workflow for WASM.

use super::config::JsCalibrationConfig;
use super::quote::JsMarketQuote;
use super::report::JsCalibrationReport;
use crate::core::dates::Date;
use crate::core::market_data::context::JsMarketContext;
use finstack_valuations::calibration::simple_calibration::SimpleCalibration;
use finstack_valuations::calibration::MarketQuote;
use wasm_bindgen::prelude::*;

/// Simple one-shot calibration workflow.
#[wasm_bindgen(js_name = SimpleCalibration)]
pub struct JsSimpleCalibration {
    inner: SimpleCalibration,
}

#[wasm_bindgen(js_class = SimpleCalibration)]
impl JsSimpleCalibration {
    /// Create a simple calibration workflow.
    #[wasm_bindgen(constructor)]
    pub fn new(
        base_date: &Date,
        base_currency: &str,
        config: Option<JsCalibrationConfig>,
    ) -> Result<JsSimpleCalibration, JsValue> {
        let ccy: finstack_core::currency::Currency = base_currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", base_currency)))?;

        let cfg = config
            .map(|c| c.inner())
            .unwrap_or_else(finstack_valuations::calibration::CalibrationConfig::default);

        let inner = SimpleCalibration::new(base_date.inner(), ccy).with_config(cfg);

        Ok(Self { inner })
    }

    /// Add entity seniority mapping.
    #[wasm_bindgen(js_name = addEntitySeniority)]
    pub fn add_entity_seniority(
        &mut self,
        _entity: &str,
        _seniority: &str,
    ) -> Result<(), JsValue> {
        // SimpleCalibration doesn't implement Clone, so we need to rebuild
        // Store the entity seniority for use during calibrate
        // For now, just return an error suggesting to pass config during construction
        Err(JsValue::from_str(
            "addEntitySeniority not supported in WASM; pass entity_seniority in CalibrationConfig during construction",
        ))
    }

    /// Calibrate to market quotes.
    #[wasm_bindgen]
    pub fn calibrate(&self, quotes: Vec<JsMarketQuote>) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<MarketQuote> = quotes.iter().map(|q| q.inner()).collect();

        let (market, report) = self
            .inner
            .calibrate(&rust_quotes)
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [market, report] array
        let result = js_sys::Array::new();
        result.push(&JsMarketContext::from_owned(market).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}


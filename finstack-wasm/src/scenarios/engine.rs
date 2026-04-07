//! Scenario execution engine bindings for WASM.

use crate::core::dates::calendar::{get_calendar, resolve_calendar_ref, JsCalendar};
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::scenarios::reports::JsApplicationReport;
use crate::scenarios::spec::JsScenarioSpec;
use crate::statements::types::JsFinancialModelSpec;
use crate::valuations::instruments::extract_instrument;
use finstack_scenarios::engine::ScenarioEngine;
use finstack_scenarios::spec::RateBindingSpec;
use finstack_scenarios::ExecutionContext;
use finstack_scenarios::NodeId;
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use indexmap::IndexMap;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Orchestrates the deterministic application of a ScenarioSpec.
///
/// The engine is intentionally lightweight: it does not own any state and can
/// be cloned or reused freely. All mutable inputs are supplied via ExecutionContext.
#[wasm_bindgen]
pub struct JsScenarioEngine {
    inner: ScenarioEngine,
}

impl Default for JsScenarioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsScenarioEngine {
    /// Create a new scenario engine with default settings.
    ///
    /// # Returns
    /// Scenario engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsScenarioEngine {
        JsScenarioEngine {
            inner: ScenarioEngine::new(),
        }
    }

    /// Compose multiple scenarios into a single deterministic spec.
    ///
    /// Operations are sorted by (priority, declaration_index); conflicts use last-wins.
    ///
    /// # Arguments
    /// * `scenarios` - Array of scenario specifications (as JSON)
    ///
    /// # Returns
    /// Combined scenario specification
    #[wasm_bindgen]
    pub fn compose(&self, scenarios: &JsValue) -> Result<JsScenarioSpec, JsValue> {
        let specs: Vec<finstack_scenarios::ScenarioSpec> =
            serde_wasm_bindgen::from_value(scenarios.clone())
                .map_err(|e| JsValue::from_str(&format!("Failed to parse scenarios: {}", e)))?;

        let composed = self.inner.compose(specs);
        Ok(JsScenarioSpec::from(composed))
    }

    /// Apply a scenario specification to the execution context.
    ///
    /// Operations are applied in this order:
    /// 1. Market data (FX, equities, vol surfaces, curves, base correlation)
    /// 2. Rate bindings update (if configured)
    /// 3. Statement forecast adjustments
    /// 4. Statement re-evaluation
    ///
    /// # Arguments
    /// * `spec` - Scenario specification to apply
    /// * `context` - Execution context that supplies market data, statements, and configuration
    ///
    /// # Returns
    /// Application report summarizing operations applied and any warnings
    #[wasm_bindgen]
    pub fn apply(
        &self,
        spec: &JsScenarioSpec,
        context: &mut JsExecutionContext,
    ) -> Result<JsApplicationReport, JsValue> {
        // Create the Rust execution context by mutably borrowing the internals
        let instruments = context.rust_instruments.as_mut();
        let rate_bindings = context.rate_bindings.clone();
        let calendar = match context.calendar.as_ref() {
            Some(code) => Some(resolve_calendar_ref(code)?),
            None => None,
        };

        let mut rust_ctx = ExecutionContext {
            market: &mut context.inner_market,
            model: &mut context.inner_model,
            instruments,
            rate_bindings,
            calendar,
            as_of: context.as_of,
        };

        // Apply the scenario
        let report = self
            .inner
            .apply(&spec.inner, &mut rust_ctx)
            .map_err(|e| JsValue::from_str(&format!("Failed to apply scenario: {}", e)))?;

        // Update the as_of date in case it was changed
        context.as_of = rust_ctx.as_of;

        Ok(JsApplicationReport::from(report))
    }
}

/// Execution context for scenario application.
///
/// The context pins all mutable state that a scenario can touch — market data,
/// statement models, instruments, rate bindings, and the current valuation date.
#[wasm_bindgen]
pub struct JsExecutionContext {
    inner_market: finstack_core::market_data::context::MarketContext,
    inner_model: finstack_statements::FinancialModelSpec,
    instruments: Option<Vec<JsValue>>,
    rust_instruments: Option<Vec<Box<dyn Instrument>>>,
    rate_bindings: Option<IndexMap<NodeId, RateBindingSpec>>,
    calendar: Option<String>,
    as_of: finstack_core::dates::Date,
}

#[wasm_bindgen]
impl JsExecutionContext {
    fn convert_instruments(
        instruments: &Option<Vec<JsValue>>,
    ) -> Result<Option<Vec<Box<dyn Instrument>>>, JsValue> {
        if let Some(list) = instruments {
            let mut rust_instruments = Vec::with_capacity(list.len());
            for value in list {
                rust_instruments.push(extract_instrument(value)?);
            }
            Ok(Some(rust_instruments))
        } else {
            Ok(None)
        }
    }

    fn convert_rate_bindings(
        value: &JsValue,
    ) -> Result<Option<IndexMap<NodeId, RateBindingSpec>>, JsValue> {
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }

        // Preferred shape: { [nodeId]: RateBindingSpec }
        if let Ok(map) =
            serde_wasm_bindgen::from_value::<IndexMap<String, RateBindingSpec>>(value.clone())
        {
            let converted = map.into_iter().map(|(k, v)| (k.into(), v)).collect();
            return Ok(Some(converted));
        }

        // Accept arrays of RateBindingSpec by deriving node_id keys.
        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<RateBindingSpec>>(value.clone()) {
            let mapped = list
                .into_iter()
                .map(|spec| (spec.node_id.clone(), spec))
                .collect();
            return Ok(Some(mapped));
        }

        Err(JsValue::from_str(
            "Failed to parse rate bindings: expected IndexMap<NodeId, RateBindingSpec> or Vec<RateBindingSpec>",
        ))
    }

    /// Create a new execution context.
    ///
    /// # Arguments
    /// * `market` - Market data context
    /// * `model` - Financial statements model
    /// * `as_of` - Valuation date
    /// * `instruments` - Optional array of instrument wrappers
    /// * `rate_bindings` - Optional mapping from statement node IDs to curve IDs
    /// * `calendar` - Optional holiday calendar reference
    ///
    /// # Returns
    /// Execution context instance
    #[wasm_bindgen(constructor)]
    pub fn new(
        market: &JsMarketContext,
        model: &JsFinancialModelSpec,
        as_of: &FsDate,
        instruments: Option<Array>,
        rate_bindings: JsValue,
        calendar: Option<JsCalendar>,
    ) -> Result<JsExecutionContext, JsValue> {
        // Access inner via the public method
        let market_inner = market.inner().clone();
        let model_inner = model.inner.clone();
        let instrument_values = instruments.map(|arr| arr.iter().collect::<Vec<JsValue>>());
        let rust_instruments = JsExecutionContext::convert_instruments(&instrument_values)?;
        let bindings = JsExecutionContext::convert_rate_bindings(&rate_bindings)?;
        let calendar_code = calendar.map(|cal| cal.code());

        Ok(JsExecutionContext {
            inner_market: market_inner,
            inner_model: model_inner,
            instruments: instrument_values,
            rust_instruments,
            rate_bindings: bindings,
            calendar: calendar_code,
            as_of: as_of.inner(),
        })
    }

    /// Get the current valuation date.
    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> FsDate {
        FsDate::from_core(self.as_of)
    }

    /// Set the valuation date.
    #[wasm_bindgen(setter, js_name = asOf)]
    pub fn set_as_of(&mut self, date: &FsDate) {
        self.as_of = date.inner();
    }

    /// Get the market context.
    #[wasm_bindgen(getter)]
    pub fn market(&self) -> JsMarketContext {
        JsMarketContext::from_owned(self.inner_market.clone())
    }

    /// Get the financial model.
    #[wasm_bindgen(getter)]
    pub fn model(&self) -> JsFinancialModelSpec {
        JsFinancialModelSpec::new(self.inner_model.clone())
    }

    /// Get the instruments list (if configured).
    #[wasm_bindgen(getter)]
    pub fn instruments(&self) -> Option<Array> {
        self.instruments.as_ref().map(|list| {
            let arr = Array::new();
            for value in list {
                arr.push(value);
            }
            arr
        })
    }

    /// Set the instruments list (rebuilds the internal Rust handles).
    #[wasm_bindgen(setter)]
    pub fn set_instruments(&mut self, instruments: Option<Array>) -> Result<(), JsValue> {
        self.instruments = instruments.map(|arr| arr.iter().collect::<Vec<JsValue>>());
        self.rust_instruments = JsExecutionContext::convert_instruments(&self.instruments)?;
        Ok(())
    }

    /// Get the rate bindings (if configured).
    #[wasm_bindgen(getter, js_name = rateBindings)]
    pub fn rate_bindings(&self) -> Result<JsValue, JsValue> {
        if let Some(bindings) = &self.rate_bindings {
            serde_wasm_bindgen::to_value(bindings).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize rate bindings: {}", e))
            })
        } else {
            Ok(JsValue::undefined())
        }
    }

    /// Set the rate bindings.
    #[wasm_bindgen(setter, js_name = rateBindings)]
    pub fn set_rate_bindings(&mut self, bindings: &JsValue) -> Result<(), JsValue> {
        self.rate_bindings = JsExecutionContext::convert_rate_bindings(bindings)?;
        Ok(())
    }

    /// Get the holiday calendar (if configured).
    #[wasm_bindgen(getter)]
    pub fn calendar(&self) -> Result<Option<JsCalendar>, JsValue> {
        self.calendar
            .as_ref()
            .map(|code| get_calendar(code))
            .transpose()
    }

    /// Set the holiday calendar by wrapper.
    #[wasm_bindgen(setter)]
    pub fn set_calendar(&mut self, calendar: Option<JsCalendar>) {
        self.calendar = calendar.map(|cal| cal.code());
    }
}

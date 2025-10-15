//! Scenario execution engine bindings for WASM.

use crate::core::dates::FsDate;
use crate::core::market_data::MarketContext;
use crate::scenarios::reports::JsApplicationReport;
use crate::scenarios::spec::JsScenarioSpec;
use crate::statements::types::JsFinancialModelSpec;
use finstack_scenarios::engine::ScenarioEngine;
use finstack_scenarios::ExecutionContext;
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
        let specs: Vec<finstack_scenarios::ScenarioSpec> = serde_wasm_bindgen::from_value(scenarios.clone())
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
        let mut rust_ctx = ExecutionContext {
            market: &mut context.inner_market,
            model: &mut context.inner_model,
            instruments: None,
            rate_bindings: None,
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
/// statement models, and the current valuation date.
#[wasm_bindgen]
pub struct JsExecutionContext {
    inner_market: finstack_core::market_data::MarketContext,
    inner_model: finstack_statements::FinancialModelSpec,
    as_of: finstack_core::dates::Date,
}

#[wasm_bindgen]
impl JsExecutionContext {
    /// Create a new execution context.
    ///
    /// # Arguments
    /// * `market` - Market data context
    /// * `model` - Financial statements model
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// Execution context instance
    #[wasm_bindgen(constructor)]
    pub fn new(market: &MarketContext, model: &JsFinancialModelSpec, as_of: &FsDate) -> Result<JsExecutionContext, JsValue> {
        // Access inner via the public method
        let market_inner = market.inner().clone();
        let model_inner = model.inner.clone();
        
        Ok(JsExecutionContext {
            inner_market: market_inner,
            inner_model: model_inner,
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
    pub fn market(&self) -> MarketContext {
        MarketContext::from_owned(self.inner_market.clone())
    }

    /// Get the financial model.
    #[wasm_bindgen(getter)]
    pub fn model(&self) -> JsFinancialModelSpec {
        JsFinancialModelSpec::new(self.inner_model.clone())
    }
}



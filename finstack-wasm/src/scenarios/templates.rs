//! WASM bindings for scenario template metadata, registry, and builder.
//!
//! Exposes the template registry for discovering and constructing
//! historical stress test scenarios.

use crate::core::error::js_error;
use crate::scenarios::spec::JsScenarioSpec;
use finstack_scenarios::templates::{
    AssetClass, ScenarioSpecBuilder, Severity, TemplateMetadata, TemplateRegistry,
};
use wasm_bindgen::prelude::*;

/// Severity classification for stress scenarios.
#[wasm_bindgen(js_name = Severity)]
#[derive(Clone, Copy)]
pub enum JsSeverity {
    /// Mild stress with limited market dislocation.
    Mild = "mild",
    /// Moderate stress with broader but contained impact.
    Moderate = "moderate",
    /// Severe systemic stress with large cross-asset dislocations.
    Severe = "severe",
}

impl From<JsSeverity> for Severity {
    fn from(js: JsSeverity) -> Self {
        match js {
            JsSeverity::Mild => Severity::Mild,
            JsSeverity::Moderate => Severity::Moderate,
            JsSeverity::Severe => Severity::Severe,
            _ => Severity::Moderate,
        }
    }
}

impl From<Severity> for JsSeverity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Mild => JsSeverity::Mild,
            Severity::Moderate => JsSeverity::Moderate,
            Severity::Severe => JsSeverity::Severe,
        }
    }
}

/// Asset class categories affected by a stress template.
#[wasm_bindgen(js_name = AssetClass)]
#[derive(Clone, Copy)]
pub enum JsAssetClass {
    /// Interest rates and fixed income.
    Rates = "rates",
    /// Credit spreads and default risk.
    Credit = "credit",
    /// Equity prices and dividends.
    Equity = "equity",
    /// Foreign exchange rates.
    FX = "fx",
    /// Implied and realized volatility.
    Volatility = "volatility",
    /// Commodity prices.
    Commodity = "commodity",
}

impl From<JsAssetClass> for AssetClass {
    fn from(js: JsAssetClass) -> Self {
        match js {
            JsAssetClass::Rates => AssetClass::Rates,
            JsAssetClass::Credit => AssetClass::Credit,
            JsAssetClass::Equity => AssetClass::Equity,
            JsAssetClass::FX => AssetClass::FX,
            JsAssetClass::Volatility => AssetClass::Volatility,
            JsAssetClass::Commodity => AssetClass::Commodity,
            _ => AssetClass::Rates,
        }
    }
}

impl From<AssetClass> for JsAssetClass {
    fn from(ac: AssetClass) -> Self {
        match ac {
            AssetClass::Rates => JsAssetClass::Rates,
            AssetClass::Credit => JsAssetClass::Credit,
            AssetClass::Equity => JsAssetClass::Equity,
            AssetClass::FX => JsAssetClass::FX,
            AssetClass::Volatility => JsAssetClass::Volatility,
            AssetClass::Commodity => JsAssetClass::Commodity,
        }
    }
}

/// Metadata describing a historical stress test template.
#[wasm_bindgen(js_name = TemplateMetadata)]
pub struct JsTemplateMetadata {
    inner: TemplateMetadata,
}

#[wasm_bindgen(js_class = TemplateMetadata)]
impl JsTemplateMetadata {
    /// Stable template identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Human-readable template name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Description of the historical event.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    /// Primary date associated with the historical event (ISO string).
    #[wasm_bindgen(getter, js_name = eventDate)]
    pub fn event_date(&self) -> String {
        self.inner.event_date.to_string()
    }

    /// Severity classification.
    #[wasm_bindgen(getter)]
    pub fn severity(&self) -> JsSeverity {
        JsSeverity::from(self.inner.severity)
    }

    /// Freeform tags for filtering.
    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    /// Component template IDs.
    #[wasm_bindgen(getter)]
    pub fn components(&self) -> Vec<String> {
        self.inner.components.clone()
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize TemplateMetadata: {e}")))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "TemplateMetadata(id='{}', severity={:?})",
            self.inner.id, self.inner.severity
        )
    }
}

/// Builder for constructing `ScenarioSpec` values with parameterized overrides.
///
/// Template factories return builders pre-populated with conventional identifiers.
/// Override them to match your own market data before calling `build()`.
#[wasm_bindgen(js_name = ScenarioSpecBuilder)]
pub struct JsScenarioSpecBuilder {
    inner: ScenarioSpecBuilder,
}

#[wasm_bindgen(js_class = ScenarioSpecBuilder)]
impl JsScenarioSpecBuilder {
    /// Create a new builder with a scenario identifier.
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: ScenarioSpecBuilder::new(id),
        }
    }

    /// Set the human-readable scenario name.
    #[wasm_bindgen]
    pub fn name(self, name: &str) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: self.inner.name(name),
        }
    }

    /// Set the optional scenario description.
    #[wasm_bindgen]
    pub fn description(self, description: &str) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: self.inner.description(description),
        }
    }

    /// Set composition priority (lower = applied first).
    #[wasm_bindgen]
    pub fn priority(self, priority: i32) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: self.inner.priority(priority),
        }
    }

    /// Override a curve identifier.
    #[wasm_bindgen(js_name = overrideCurve)]
    pub fn override_curve(self, default_id: &str, user_id: &str) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: self.inner.override_curve(default_id, user_id),
        }
    }

    /// Override an equity identifier.
    #[wasm_bindgen(js_name = overrideEquity)]
    pub fn override_equity(self, default_id: &str, user_id: &str) -> JsScenarioSpecBuilder {
        JsScenarioSpecBuilder {
            inner: self.inner.override_equity(default_id, user_id),
        }
    }

    /// Build the final validated ScenarioSpec.
    #[wasm_bindgen]
    pub fn build(self) -> Result<JsScenarioSpec, JsValue> {
        let spec = self
            .inner
            .build()
            .map_err(|e| js_error(format!("Failed to build scenario: {e}")))?;
        Ok(JsScenarioSpec::from(spec))
    }
}

/// Registry of template metadata and builder factories for stress test scenarios.
///
/// Use `TemplateRegistry.withBuiltins()` to get a registry preloaded with
/// historical stress templates (GFC 2008, COVID 2020, etc.).
#[wasm_bindgen(js_name = TemplateRegistry)]
pub struct JsTemplateRegistry {
    inner: TemplateRegistry,
}

impl Default for JsTemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = TemplateRegistry)]
impl JsTemplateRegistry {
    /// Create an empty registry.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsTemplateRegistry {
        JsTemplateRegistry {
            inner: TemplateRegistry::new(),
        }
    }

    /// Create a registry preloaded with built-in historical stress templates.
    #[wasm_bindgen(js_name = withBuiltins)]
    pub fn with_builtins() -> Result<JsTemplateRegistry, JsValue> {
        let inner = TemplateRegistry::with_embedded_builtins()
            .map_err(|e| js_error(format!("Failed to load builtins: {e}")))?;
        Ok(JsTemplateRegistry { inner })
    }

    /// Register a template from a runtime JSON string.
    #[wasm_bindgen(js_name = registerJsonTemplate)]
    pub fn register_json_template(&mut self, name: &str, json: &str) -> Result<(), JsValue> {
        self.inner
            .register_json_template_str(name, json)
            .map_err(|e| js_error(format!("Failed to register JSON template: {e}")))
    }

    /// Get template metadata by identifier.
    #[wasm_bindgen]
    pub fn get(&self, id: &str) -> Option<JsTemplateMetadata> {
        self.inner
            .get(id)
            .map(|entry| JsTemplateMetadata {
                inner: entry.metadata().clone(),
            })
    }

    /// List all registered template metadata.
    #[wasm_bindgen]
    pub fn list(&self) -> Vec<JsTemplateMetadata> {
        self.inner
            .list()
            .into_iter()
            .map(|m| JsTemplateMetadata { inner: m.clone() })
            .collect()
    }

    /// Filter templates by tag.
    #[wasm_bindgen(js_name = filterByTag)]
    pub fn filter_by_tag(&self, tag: &str) -> Vec<JsTemplateMetadata> {
        self.inner
            .filter_by_tag(tag)
            .into_iter()
            .map(|m| JsTemplateMetadata { inner: m.clone() })
            .collect()
    }

    /// Filter templates by asset class.
    #[wasm_bindgen(js_name = filterByAssetClass)]
    pub fn filter_by_asset_class(&self, asset_class: JsAssetClass) -> Vec<JsTemplateMetadata> {
        self.inner
            .filter_by_asset_class(asset_class.into())
            .into_iter()
            .map(|m| JsTemplateMetadata { inner: m.clone() })
            .collect()
    }

    /// Filter templates by severity.
    #[wasm_bindgen(js_name = filterBySeverity)]
    pub fn filter_by_severity(&self, severity: JsSeverity) -> Vec<JsTemplateMetadata> {
        self.inner
            .filter_by_severity(severity.into())
            .into_iter()
            .map(|m| JsTemplateMetadata { inner: m.clone() })
            .collect()
    }

    /// Get a fresh builder for a registered template.
    #[wasm_bindgen(js_name = getBuilder)]
    pub fn get_builder(&self, id: &str) -> Option<JsScenarioSpecBuilder> {
        self.inner
            .get(id)
            .map(|entry| JsScenarioSpecBuilder {
                inner: entry.builder(),
            })
    }

    /// Get a fresh builder for a specific component of a registered template.
    #[wasm_bindgen(js_name = getComponentBuilder)]
    pub fn get_component_builder(
        &self,
        template_id: &str,
        component_id: &str,
    ) -> Option<JsScenarioSpecBuilder> {
        self.inner.get(template_id).and_then(|entry| {
            entry.component(component_id).map(|builder| {
                JsScenarioSpecBuilder { inner: builder }
            })
        })
    }

    /// List component IDs for a registered template.
    #[wasm_bindgen(js_name = getComponentIds)]
    pub fn get_component_ids(&self, template_id: &str) -> Option<Vec<String>> {
        self.inner.get(template_id).map(|entry| {
            entry.component_ids().into_iter().map(String::from).collect()
        })
    }
}

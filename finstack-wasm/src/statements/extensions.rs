//! Extension system for statements.

use crate::statements::evaluator::JsResults;
use crate::statements::types::JsFinancialModelSpec;
use finstack_statements::extensions::{
    CorkscrewExtension, CreditScorecardExtension, ExtensionContext, ExtensionMetadata,
    ExtensionRegistry, ExtensionResult, ExtensionStatus,
};
use wasm_bindgen::prelude::*;

/// Extension metadata.
///
/// Contains information about an extension including name, version, and description.
#[wasm_bindgen]
pub struct JsExtensionMetadata {
    inner: ExtensionMetadata,
}

#[wasm_bindgen]
impl JsExtensionMetadata {
    /// Create extension metadata.
    ///
    /// # Arguments
    /// * `name` - Unique extension name
    /// * `version` - Semantic version
    /// * `description` - Optional human-readable description
    /// * `author` - Optional extension author
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: String,
        version: String,
        description: Option<String>,
        author: Option<String>,
    ) -> JsExtensionMetadata {
        JsExtensionMetadata {
            inner: ExtensionMetadata {
                name,
                version,
                description,
                author,
            },
        }
    }

    /// Get extension name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get extension version.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.inner.version.clone()
    }

    /// Get extension description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Get extension author.
    #[wasm_bindgen(getter)]
    pub fn author(&self) -> Option<String> {
        self.inner.author.clone()
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ExtensionMetadata(name='{}', version='{}')",
            self.inner.name, self.inner.version
        )
    }
}

/// Extension execution status.
///
/// Represents the outcome of an extension execution.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct JsExtensionStatus {
    inner: ExtensionStatus,
}

#[wasm_bindgen]
#[allow(non_snake_case)]
impl JsExtensionStatus {
    /// Success - extension executed successfully.
    #[wasm_bindgen(getter)]
    pub fn SUCCESS() -> JsExtensionStatus {
        JsExtensionStatus {
            inner: ExtensionStatus::Success,
        }
    }

    /// Failed - extension execution failed.
    #[wasm_bindgen(getter)]
    pub fn FAILED() -> JsExtensionStatus {
        JsExtensionStatus {
            inner: ExtensionStatus::Failed,
        }
    }

    /// Not implemented - extension is not yet implemented.
    #[wasm_bindgen(getter)]
    pub fn NOT_IMPLEMENTED() -> JsExtensionStatus {
        JsExtensionStatus {
            inner: ExtensionStatus::NotImplemented,
        }
    }

    /// Skipped - extension was skipped.
    #[wasm_bindgen(getter)]
    pub fn SKIPPED() -> JsExtensionStatus {
        JsExtensionStatus {
            inner: ExtensionStatus::Skipped,
        }
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Extension execution result.
///
/// Contains the status, message, and optional data from an extension execution.
#[wasm_bindgen]
pub struct JsExtensionResult {
    inner: ExtensionResult,
}

#[wasm_bindgen]
impl JsExtensionResult {
    /// Create a success result.
    ///
    /// # Arguments
    /// * `message` - Success message
    #[wasm_bindgen]
    pub fn success(message: String) -> JsExtensionResult {
        JsExtensionResult {
            inner: ExtensionResult::success(message),
        }
    }

    /// Create a failure result.
    ///
    /// # Arguments
    /// * `message` - Failure message
    #[wasm_bindgen]
    pub fn failure(message: String) -> JsExtensionResult {
        JsExtensionResult {
            inner: ExtensionResult::failure(message),
        }
    }

    /// Create a skipped result.
    ///
    /// # Arguments
    /// * `message` - Skip reason
    #[wasm_bindgen]
    pub fn skipped(message: String) -> JsExtensionResult {
        JsExtensionResult {
            inner: ExtensionResult::skipped(message),
        }
    }

    /// Get execution status.
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> JsExtensionStatus {
        JsExtensionStatus {
            inner: self.inner.status,
        }
    }

    /// Get result message.
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.message.clone()
    }

    /// Get result data.
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.data)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize data: {}", e)))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ExtensionResult(status={:?}, message='{}')",
            self.inner.status, self.inner.message
        )
    }
}

impl JsExtensionResult {
    fn new(inner: ExtensionResult) -> Self {
        Self { inner }
    }
}

/// Extension registry.
///
/// Manages and executes extensions for financial models.
#[wasm_bindgen]
pub struct JsExtensionRegistry {
    inner: ExtensionRegistry,
}

impl Default for JsExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsExtensionRegistry {
    /// Create a new extension registry.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsExtensionRegistry {
        JsExtensionRegistry {
            inner: ExtensionRegistry::new(),
        }
    }

    /// Execute all registered extensions.
    ///
    /// # Arguments
    /// * `model` - Financial model
    /// * `results` - Evaluation results
    ///
    /// # Returns
    /// JavaScript object mapping extension names to results
    #[wasm_bindgen(js_name = executeAll)]
    pub fn execute_all(
        &mut self,
        model: &JsFinancialModelSpec,
        results: &JsResults,
    ) -> Result<JsValue, JsValue> {
        let context = ExtensionContext::new(&model.inner, &results.inner);
        let extension_results = self
            .inner
            .execute_all(&context)
            .map_err(|e| JsValue::from_str(&format!("Extension execution failed: {}", e)))?;

        // Convert to JavaScript object
        let obj = js_sys::Object::new();
        for (name, result) in extension_results {
            let js_result = JsExtensionResult::new(result);
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str(&name),
                &serde_wasm_bindgen::to_value(&js_result.inner)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?,
            )?;
        }
        Ok(JsValue::from(obj))
    }
}

/// Corkscrew extension.
///
/// Validates balance sheet roll-forward (opening + changes = closing).
///
/// Note: Configuration is not yet supported in WASM bindings.
/// Use the default constructor which creates a new extension with default settings.
#[wasm_bindgen]
pub struct JsCorkscrewExtension {
    _inner: CorkscrewExtension,
}

impl Default for JsCorkscrewExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsCorkscrewExtension {
    /// Create a new corkscrew extension with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCorkscrewExtension {
        JsCorkscrewExtension {
            _inner: CorkscrewExtension::new(),
        }
    }
}

/// Credit scorecard extension.
///
/// Assigns credit ratings based on financial metrics.
///
/// Note: Configuration is not yet supported in WASM bindings.
/// Use the default constructor which creates a new extension with default settings.
#[wasm_bindgen]
pub struct JsCreditScorecardExtension {
    _inner: CreditScorecardExtension,
}

impl Default for JsCreditScorecardExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsCreditScorecardExtension {
    /// Create a new credit scorecard extension with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCreditScorecardExtension {
        JsCreditScorecardExtension {
            _inner: CreditScorecardExtension::new(),
        }
    }
}

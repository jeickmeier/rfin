//! Extension registry for managing and executing extensions.

use super::plugin::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};
use crate::error::Result;
use indexmap::IndexMap;

/// Registry for managing extensions.
///
/// The registry stores registered extensions and provides methods to execute them
/// against models and results.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_statements::extensions::ExtensionRegistry;
/// use finstack_statements::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};
///
/// # struct MyExtension;
/// # impl Extension for MyExtension {
/// #     fn metadata(&self) -> ExtensionMetadata {
/// #         ExtensionMetadata { name: "my_extension".into(), version: "0.1.0".into(), description: None, author: None }
/// #     }
/// #     fn execute(&mut self, _context: &ExtensionContext) -> finstack_statements::Result<ExtensionResult> {
/// #         Ok(ExtensionResult::success("ok"))
/// #     }
/// # }
/// # fn main() -> finstack_statements::Result<()> {
/// let mut registry = ExtensionRegistry::new();
/// registry.register(Box::new(MyExtension))?;
///
/// // Execute all enabled extensions
/// # let context: ExtensionContext = unimplemented!("build ExtensionContext from model/results");
/// let results = registry.execute_all(&context)?;
/// # let _ = results;
/// # Ok(())
/// # }
/// ```
pub struct ExtensionRegistry {
    /// Registered extensions by name
    extensions: IndexMap<String, Box<dyn Extension>>,

    /// Execution order (extension names in order)
    execution_order: Vec<String>,
}

impl ExtensionRegistry {
    /// Create a new extension registry.
    pub fn new() -> Self {
        Self {
            extensions: IndexMap::new(),
            execution_order: Vec::new(),
        }
    }

    /// Register an extension.
    ///
    /// # Arguments
    ///
    /// * `extension` - The extension to register
    ///
    /// # Returns
    ///
    /// Returns an error if an extension with the same name is already registered.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_statements::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult, ExtensionRegistry};
    ///
    /// # fn main() -> finstack_statements::Result<()> {
    /// # struct MyExtension;
    /// # impl Extension for MyExtension {
    /// #     fn metadata(&self) -> ExtensionMetadata {
    /// #         ExtensionMetadata { name: "my_extension".into(), version: "0.1.0".into(), description: None, author: None }
    /// #     }
    /// #     fn execute(&mut self, _context: &ExtensionContext) -> finstack_statements::Result<ExtensionResult> {
    /// #         Ok(ExtensionResult::success("ok"))
    /// #     }
    /// # }
    /// let mut registry = ExtensionRegistry::new();
    /// registry.register(Box::new(MyExtension))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register(&mut self, extension: Box<dyn Extension>) -> Result<()> {
        let metadata = extension.metadata();
        let name = metadata.name.clone();

        if self.extensions.contains_key(&name) {
            return Err(crate::error::Error::invalid_input(format!(
                "Extension '{}' is already registered",
                name
            )));
        }

        self.extensions.insert(name.clone(), extension);
        self.execution_order.push(name);

        Ok(())
    }

    /// Get an extension by name.
    pub fn get(&self, name: &str) -> Option<&dyn Extension> {
        self.extensions.get(name).map(|e| e.as_ref())
    }

    /// Get a mutable reference to an extension by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn Extension>> {
        self.extensions.get_mut(name)
    }

    /// Check if an extension is registered.
    pub fn has(&self, name: &str) -> bool {
        self.extensions.contains_key(name)
    }

    /// List all registered extension names.
    pub fn list(&self) -> Vec<String> {
        self.extensions.keys().cloned().collect()
    }

    /// Get metadata for all registered extensions.
    pub fn list_metadata(&self) -> Vec<ExtensionMetadata> {
        self.extensions.values().map(|ext| ext.metadata()).collect()
    }

    /// Iterate over all registered extensions.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn Extension)> {
        self.extensions
            .iter()
            .map(|(name, ext)| (name.as_str(), ext.as_ref()))
    }

    /// Set the execution order.
    ///
    /// # Arguments
    ///
    /// * `order` - Vector of extension names in the desired execution order
    ///
    /// # Returns
    ///
    /// Returns an error if any extension name is not registered.
    pub fn set_execution_order(&mut self, order: Vec<String>) -> Result<()> {
        // Validate all extensions in order are registered
        for name in &order {
            if !self.extensions.contains_key(name) {
                return Err(crate::error::Error::invalid_input(format!(
                    "Extension '{}' is not registered",
                    name
                )));
            }
        }

        // Warn about registered extensions that will be skipped
        for name in self.extensions.keys() {
            if !order.contains(name) {
                tracing::warn!(
                    extension = name.as_str(),
                    "Registered extension not included in execution order — it will not run"
                );
            }
        }

        self.execution_order = order;
        Ok(())
    }

    /// Execute a specific extension.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the extension to execute
    /// * `context` - Extension context
    ///
    /// # Returns
    ///
    /// Returns the extension result or an error if the extension is not found.
    pub fn execute(&mut self, name: &str, context: &ExtensionContext) -> Result<ExtensionResult> {
        let _span = tracing::info_span!(
            "statements.extension.execute",
            extension = name,
            has_runtime_config = context.config.is_some()
        )
        .entered();
        let extension = self.extensions.get_mut(name).ok_or_else(|| {
            crate::error::Error::invalid_input(format!("Extension '{}' not found", name))
        })?;

        if !extension.is_enabled() {
            return Ok(ExtensionResult::skipped("Extension is disabled"));
        }

        if let Some(config) = context.config {
            extension.validate_config(config)?;
        }

        extension.execute(context)
    }

    /// Execute all registered extensions in order.
    ///
    /// # Arguments
    ///
    /// * `context` - Extension context
    ///
    /// # Returns
    ///
    /// Returns a map of extension name → result for all executed extensions.
    pub fn execute_all(
        &mut self,
        context: &ExtensionContext,
    ) -> Result<IndexMap<String, ExtensionResult>> {
        let mut results = IndexMap::new();

        for name in &self.execution_order.clone() {
            let result = self.execute(name, context)?;
            results.insert(name.clone(), result);
        }

        Ok(results)
    }

    /// Execute all enabled extensions in order, collecting results.
    ///
    /// Unlike `execute_all`, this method continues execution even if an extension fails.
    ///
    /// # Arguments
    ///
    /// * `context` - Extension context
    ///
    /// # Returns
    ///
    /// Returns a map of extension name → result, including both successes and failures.
    pub fn execute_all_safe(
        &mut self,
        context: &ExtensionContext,
    ) -> IndexMap<String, Result<ExtensionResult>> {
        let mut results = IndexMap::new();

        for name in &self.execution_order.clone() {
            let result = self.execute(name, context);
            results.insert(name.clone(), result);
        }

        results
    }

    /// Get the number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// Clear all registered extensions.
    pub fn clear(&mut self) {
        self.extensions.clear();
        self.execution_order.clear();
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::extensions::plugin::ExtensionMetadata;
    use crate::types::FinancialModelSpec;

    struct TestExtension {
        name: String,
        enabled: bool,
    }

    impl TestExtension {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                enabled: true,
            }
        }
    }

    impl Extension for TestExtension {
        fn metadata(&self) -> ExtensionMetadata {
            ExtensionMetadata {
                name: self.name.clone(),
                version: "1.0.0".into(),
                description: Some("Test extension".into()),
                author: None,
            }
        }

        fn execute(&mut self, _context: &ExtensionContext) -> Result<ExtensionResult> {
            Ok(ExtensionResult::success(format!(
                "{} executed successfully",
                self.name
            )))
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }
    }

    struct ValidatingExtension;

    impl Extension for ValidatingExtension {
        fn metadata(&self) -> ExtensionMetadata {
            ExtensionMetadata {
                name: "validating".into(),
                version: "1.0.0".into(),
                description: None,
                author: None,
            }
        }

        fn execute(&mut self, _context: &ExtensionContext) -> Result<ExtensionResult> {
            Ok(ExtensionResult::success("validated"))
        }

        fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
            if config.get("ok").and_then(|value| value.as_bool()) == Some(true) {
                Ok(())
            } else {
                Err(crate::error::Error::invalid_input("missing ok=true"))
            }
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ExtensionRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_extension() {
        let mut registry = ExtensionRegistry::new();
        let extension = Box::new(TestExtension::new("test1"));

        registry.register(extension).expect("test should succeed");

        assert_eq!(registry.len(), 1);
        assert!(registry.has("test1"));
    }

    #[test]
    fn test_duplicate_registration_error() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("test1")))
            .expect("test should succeed");

        let result = registry.register(Box::new(TestExtension::new("test1")));
        assert!(result.is_err());
    }

    #[test]
    fn test_list_extensions() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("ext1")))
            .expect("test should succeed");
        registry
            .register(Box::new(TestExtension::new("ext2")))
            .expect("test should succeed");

        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"ext1".to_string()));
        assert!(names.contains(&"ext2".to_string()));
    }

    #[test]
    fn test_list_metadata() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("ext1")))
            .expect("test should succeed");

        let metadata = registry.list_metadata();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "ext1");
        assert_eq!(metadata[0].version, "1.0.0");
    }

    #[test]
    fn test_set_execution_order() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("ext1")))
            .expect("test should succeed");
        registry
            .register(Box::new(TestExtension::new("ext2")))
            .expect("test should succeed");

        registry
            .set_execution_order(vec!["ext2".into(), "ext1".into()])
            .expect("test should succeed");

        assert_eq!(registry.execution_order, vec!["ext2", "ext1"]);
    }

    #[test]
    fn test_execution_order_invalid_extension() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("ext1")))
            .expect("test should succeed");

        let result = registry.set_execution_order(vec!["ext1".into(), "nonexistent".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(TestExtension::new("ext1")))
            .expect("test should succeed");

        assert_eq!(registry.len(), 1);

        registry.clear();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_execute_validates_runtime_config() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(ValidatingExtension))
            .expect("registration should succeed");

        let model = FinancialModelSpec::new("test", vec![]);
        let results = crate::evaluator::StatementResult::new();
        let invalid = serde_json::json!({"ok": false});
        let context = ExtensionContext::new(&model, &results).with_config(&invalid);

        let err = registry
            .execute("validating", &context)
            .expect_err("invalid config should fail");
        assert!(err.to_string().contains("missing ok=true"));
    }

    #[test]
    fn test_execute_all_passes_config_to_extensions() {
        let mut registry = ExtensionRegistry::new();
        registry
            .register(Box::new(ValidatingExtension))
            .expect("registration should succeed");
        registry
            .register(Box::new(TestExtension::new("plain")))
            .expect("registration should succeed");

        let model = FinancialModelSpec::new("test", vec![]);
        let results = crate::evaluator::StatementResult::new();

        // With no config, all extensions execute successfully
        let context_no_config = ExtensionContext::new(&model, &results);
        let all_results = registry
            .execute_all(&context_no_config)
            .expect("no config should pass for all extensions");
        assert_eq!(all_results.len(), 2);

        // With a valid config, ValidatingExtension should also succeed
        let valid = serde_json::json!({"ok": true});
        let context_valid = ExtensionContext::new(&model, &results).with_config(&valid);
        let all_results = registry
            .execute_all(&context_valid)
            .expect("valid config should pass for all extensions");
        assert_eq!(all_results.len(), 2);
    }
}

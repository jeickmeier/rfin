//! Named Function Registry for deterministic financial functions.
//!
//! Provides a global registry for reusable financial computations with:
//! - Deterministic function registration and retrieval
//! - Toggle functions for conditional logic
//! - FX policy functions for currency conversions
//! - General policy functions for custom behaviors
//! - Serde-serializable parameters for all functions

use finstack_core::prelude::*;
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

// Removed duplicate FxProvider – use core trait instead

/// Parameter trait for function arguments.
/// Must be serializable, cloneable, debuggable, and thread-safe.
pub trait FunctionParam: Debug + Send + Sync {
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Clone the parameter as a boxed trait object
    fn clone_box(&self) -> Box<dyn FunctionParam>;

    /// Serialize to JSON
    fn to_json(&self) -> finstack_core::Result<String>;
}

// Use a macro to implement FunctionParam for concrete types
#[macro_export]
macro_rules! impl_function_param {
    ($t:ty) => {
        impl FunctionParam for $t {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn clone_box(&self) -> Box<dyn FunctionParam> {
                Box::new(self.clone())
            }

            fn to_json(&self) -> finstack_core::Result<String> {
                serde_json::to_string(self)
                    .map_err(|_| finstack_core::error::InputError::Invalid.into())
            }
        }
    };
}

/// Toggle function type for conditional logic.
/// Returns true/false based on input parameters.
pub type ToggleFn = Arc<dyn Fn(&dyn Any) -> finstack_core::Result<bool> + Send + Sync>;

/// FX policy function type for currency conversions.
/// Takes parameters and returns an FxProvider implementation.
pub type FxPolicyFn =
    Arc<dyn Fn(&dyn Any) -> finstack_core::Result<Box<dyn FxProvider>> + Send + Sync>;

/// General policy function type for custom behaviors.
/// Takes parameters and returns a serializable result.
pub type PolicyFn =
    Arc<dyn Fn(&dyn Any) -> finstack_core::Result<Box<dyn Any + Send + Sync>> + Send + Sync>;

/// Registry for named financial functions.
///
/// Provides centralized storage and retrieval of deterministic functions
/// used across the valuations framework.
#[derive(Default)]
pub struct FunctionRegistry {
    /// Toggle functions for conditional logic
    toggles: HashMap<String, ToggleFn>,

    /// FX policy functions for currency conversions
    fx_policies: HashMap<String, FxPolicyFn>,

    /// General policy functions for custom behaviors
    policies: HashMap<String, PolicyFn>,
}

impl FunctionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a toggle function.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the toggle
    /// * `func` - Toggle function implementation
    ///
    /// See unit tests and `examples/` for usage.
    pub fn register_toggle(&mut self, name: impl Into<String>, func: ToggleFn) {
        self.toggles.insert(name.into(), func);
    }

    /// Register an FX policy function.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the FX policy
    /// * `func` - FX policy function implementation
    pub fn register_fx_policy(&mut self, name: impl Into<String>, func: FxPolicyFn) {
        self.fx_policies.insert(name.into(), func);
    }

    /// Register a general policy function.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the policy
    /// * `func` - Policy function implementation
    pub fn register_policy(&mut self, name: impl Into<String>, func: PolicyFn) {
        self.policies.insert(name.into(), func);
    }

    /// Get a toggle function by name.
    pub fn get_toggle(&self, name: &str) -> Option<&ToggleFn> {
        self.toggles.get(name)
    }

    /// Get an FX policy function by name.
    pub fn get_fx_policy(&self, name: &str) -> Option<&FxPolicyFn> {
        self.fx_policies.get(name)
    }

    /// Get a general policy function by name.
    pub fn get_policy(&self, name: &str) -> Option<&PolicyFn> {
        self.policies.get(name)
    }

    /// Check if a toggle exists.
    pub fn has_toggle(&self, name: &str) -> bool {
        self.toggles.contains_key(name)
    }

    /// Check if an FX policy exists.
    pub fn has_fx_policy(&self, name: &str) -> bool {
        self.fx_policies.contains_key(name)
    }

    /// Check if a general policy exists.
    pub fn has_policy(&self, name: &str) -> bool {
        self.policies.contains_key(name)
    }

    /// List all registered toggle names.
    pub fn toggle_names(&self) -> Vec<String> {
        self.toggles.keys().cloned().collect()
    }

    /// List all registered FX policy names.
    pub fn fx_policy_names(&self) -> Vec<String> {
        self.fx_policies.keys().cloned().collect()
    }

    /// List all registered general policy names.
    pub fn policy_names(&self) -> Vec<String> {
        self.policies.keys().cloned().collect()
    }
}

/// Global function registry singleton.
///
/// Provides thread-safe access to the shared registry instance.
pub static FN_REGISTRY: Lazy<RwLock<FunctionRegistry>> =
    Lazy::new(|| RwLock::new(FunctionRegistry::new()));

/// Register a toggle function in the global registry.
///
/// See unit tests and `examples/` for usage.
pub fn register_toggle(name: impl Into<String>, func: ToggleFn) {
    FN_REGISTRY.write().unwrap().register_toggle(name, func);
}

/// Register an FX policy function in the global registry.
pub fn register_fx_policy(name: impl Into<String>, func: FxPolicyFn) {
    FN_REGISTRY.write().unwrap().register_fx_policy(name, func);
}

/// Register a general policy function in the global registry.
pub fn register_policy(name: impl Into<String>, func: PolicyFn) {
    FN_REGISTRY.write().unwrap().register_policy(name, func);
}

/// Get a toggle function from the global registry.
pub fn get_toggle(name: &str) -> Option<ToggleFn> {
    FN_REGISTRY.read().unwrap().get_toggle(name).cloned()
}

/// Get an FX policy function from the global registry.
pub fn get_fx_policy(name: &str) -> Option<FxPolicyFn> {
    FN_REGISTRY.read().unwrap().get_fx_policy(name).cloned()
}

/// Get a general policy function from the global registry.
pub fn get_policy(name: &str) -> Option<PolicyFn> {
    FN_REGISTRY.read().unwrap().get_policy(name).cloned()
}

/// Common toggle parameters for date-based conditions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateToggleParams {
    /// The date to check
    pub date: Date,
    /// Reference date for comparison
    pub reference: Date,
}

impl_function_param!(DateToggleParams);

/// Common FX policy parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxPolicyParams {
    /// Base currency for conversions
    pub base_currency: Currency,
    /// Optional conversion method
    pub method: Option<String>,
}

impl_function_param!(FxPolicyParams);

/// Initialize standard built-in functions.
///
/// Call this during application startup to register commonly used functions.
pub fn init_standard_functions() {
    // Register standard date toggles
    register_toggle(
        "after_date",
        Arc::new(|params| {
            let p = params
                .downcast_ref::<DateToggleParams>()
                .ok_or(finstack_core::error::InputError::Invalid)?;
            Ok(p.date > p.reference)
        }),
    );

    register_toggle(
        "before_date",
        Arc::new(|params| {
            let p = params
                .downcast_ref::<DateToggleParams>()
                .ok_or(finstack_core::error::InputError::Invalid)?;
            Ok(p.date < p.reference)
        }),
    );

    // More standard functions can be added here as needed
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct TestParams {
        value: f64,
    }

    impl_function_param!(TestParams);

    #[test]
    fn test_registry_toggle() {
        let mut registry = FunctionRegistry::new();

        registry.register_toggle(
            "is_positive",
            Arc::new(|params| {
                let p = params
                    .downcast_ref::<TestParams>()
                    .ok_or(finstack_core::error::InputError::Invalid)?;
                Ok(p.value > 0.0)
            }),
        );

        assert!(registry.has_toggle("is_positive"));
        assert!(!registry.has_toggle("nonexistent"));

        let toggle = registry.get_toggle("is_positive").unwrap();
        let params = TestParams { value: 5.0 };
        assert!(toggle(&params).unwrap());

        let params = TestParams { value: -5.0 };
        assert!(!toggle(&params).unwrap());
    }

    #[test]
    fn test_global_registry() {
        register_toggle("test_toggle", Arc::new(|_| Ok(true)));

        let toggle = get_toggle("test_toggle").unwrap();
        let params = TestParams { value: 0.0 };
        assert!(toggle(&params).unwrap());
    }

    #[test]
    fn test_standard_functions() {
        init_standard_functions();

        let toggle = get_toggle("after_date").unwrap();
        let params = DateToggleParams {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            reference: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        };
        assert!(toggle(&params).unwrap());
    }
}

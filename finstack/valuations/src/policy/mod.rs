//! Policy and function registry module.
//!
//! Provides infrastructure for deterministic function registration and management,
//! supporting toggles, FX policies, and general policy functions.

pub mod policies;
pub mod registry;

pub use registry::{
    get_fx_policy, get_policy, get_toggle, init_standard_functions, register_fx_policy,
    register_policy, register_toggle, DateToggleParams, FunctionParam, FunctionRegistry,
    FxPolicyFn, FxPolicyParams, PolicyFn, ToggleFn, FN_REGISTRY,
};
// Re-export the core FxProvider for consumers
pub use finstack_core::money::fx::FxProvider;

pub use policies::{
    calculate_dscr, calculate_interest_coverage, register_policy_functions, DSCRSweepParams,
    DSCRSweepPolicy, GridMarginParams, GridMarginPolicy, IndexFallbackPolicy,
};

pub use crate::impl_function_param;

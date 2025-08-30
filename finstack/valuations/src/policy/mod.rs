//! Policy and function registry module.
//!
//! Provides infrastructure for deterministic function registration and management,
//! supporting toggles, FX policies, and general policy functions.

pub mod registry;
pub mod policies;

pub use registry::{
    FunctionRegistry, FunctionParam,
    ToggleFn, FxPolicyFn, PolicyFn,
    FN_REGISTRY,
    register_toggle, register_fx_policy, register_policy,
    get_toggle, get_fx_policy, get_policy,
    DateToggleParams, FxPolicyParams,
    init_standard_functions,
};
// Re-export the core FxProvider for consumers
pub use finstack_core::money::fx::FxProvider;

pub use policies::{
    GridMarginPolicy,
    IndexFallbackPolicy,
    DSCRSweepPolicy,
    GridMarginParams,
    DSCRSweepParams,
    calculate_dscr,
    calculate_interest_coverage,
    register_policy_functions,
};

pub use crate::impl_function_param;

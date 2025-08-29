//! Policy and function registry module.
//!
//! Provides infrastructure for deterministic function registration and management,
//! supporting toggles, FX policies, and general policy functions.

pub mod registry;

pub use registry::{
    FunctionRegistry, FunctionParam, FxProvider,
    ToggleFn, FxPolicyFn, PolicyFn,
    FN_REGISTRY,
    register_toggle, register_fx_policy, register_policy,
    get_toggle, get_fx_policy, get_policy,
    DateToggleParams, FxPolicyParams,
    init_standard_functions,
};

pub use crate::impl_function_param;

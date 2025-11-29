//! Option pricing models and numerical methods with academic foundations.
//!
//! Provides reusable pricing models for options and derivatives, including
//! closed-form formulas, tree-based methods, volatility models, and Black-Scholes variants.
//! All implementations cite their academic sources for correctness verification.
//!
//! # Module Organization
//!
//! - [`closed_form`]: Closed-form and semi-analytical pricing formulas (Black-Scholes Greeks,
//!   Asian, Barrier, Lookback, Quanto, Heston)
//! - [`volatility`]: Volatility models (SABR) and Black-Scholes helper functions
//! - [`trees`]: Tree-based methods (Binomial, Trinomial, Multi-factor, Short-rate)
//! - [`correlation`]: Shared correlation infrastructure (copulas, recovery models, factor models)

pub mod closed_form;
pub mod correlation;
#[cfg(feature = "mc")]
pub mod monte_carlo;
pub mod trees;
pub mod volatility;

// Re-export commonly used items from submodules for convenience
pub use closed_form::{
    arithmetic_asian_call_tw, arithmetic_asian_put_tw, barrier_call_continuous,
    barrier_put_continuous, bs_call_delta, bs_call_greeks, bs_call_rho, bs_call_theta, bs_gamma,
    bs_put_delta, bs_put_greeks, bs_put_rho, bs_put_theta, bs_vega, down_in_call, down_out_call,
    fixed_strike_lookback_call, fixed_strike_lookback_put, floating_strike_lookback_call,
    floating_strike_lookback_put, geometric_asian_call, geometric_asian_put,
    heston_call_price_fourier, heston_put_price_fourier, quanto_call, quanto_call_simple,
    quanto_drift_adjustment, quanto_put, quanto_put_simple, up_in_call, up_out_call, AsianGreeks,
    AsianPriceResult, BarrierType, CallGreeks, HestonParams, PutGreeks,
};
pub use correlation::{
    joint_probabilities, ConstantRecovery, Copula, CopulaSpec, CorrelatedBernoulli,
    CorrelatedRecovery, FactorModel, FactorSpec, GaussianCopula, MultiFactorCopula,
    RandomFactorLoadingCopula, RecoveryModel, RecoverySpec, SingleFactorModel, StudentTCopula,
    TwoFactorModel,
};
pub use trees::{
    short_rate_keys, single_factor_equity_state, state_keys, two_factor_equity_rates_state,
    BarrierSpec, BarrierStyle, BinomialTree, EvolutionParams, NodeState, ShortRateModel,
    ShortRateTree, ShortRateTreeConfig, StateVariables, TreeBranching, TreeGreeks, TreeModel,
    TreeParameters, TreeType, TreeValuator, TrinomialTree, TrinomialTreeType,
};
pub use volatility::{
    d1, d1_black76, d2, d2_black76, norm_cdf, norm_pdf, SABRCalibrator, SABRModel, SABRParameters,
    SABRSmile,
};

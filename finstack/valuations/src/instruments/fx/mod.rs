//! FX instruments and FX derivatives.

/// FX barrier option module.
pub mod fx_barrier_option;
/// FX forward module.
pub mod fx_forward;
/// FX option module - Vanilla FX options.
pub mod fx_option;
/// FX spot module - FX spot trades.
pub mod fx_spot;
/// FX swap module - FX swaps with near/far legs.
pub mod fx_swap;
/// FX variance swap module.
pub mod fx_variance_swap;
/// NDF module - Non-deliverable forwards.
pub mod ndf;
/// Quanto option module - Cross-currency quanto options.
pub mod quanto_option;

// Re-export primary types
pub use fx_barrier_option::FxBarrierOption;
pub use fx_forward::FxForward;
pub use fx_option::FxOption;
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use fx_variance_swap::FxVarianceSwap;
pub use ndf::Ndf;
pub use quanto_option::QuantoOption;

pub mod builder;
pub mod metrics;
mod types;

pub use types::FxSwap;

// Provide a distinct path for types.rs to reference this builder
pub(crate) mod mod_fx_swap {
    // builder provided via derive
}

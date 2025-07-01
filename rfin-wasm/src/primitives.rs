//! WASM bindings for primitives module.

pub mod currency;
pub mod money;

// Re-export for external use
pub use currency::Currency;
pub use money::Money;

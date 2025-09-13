//! Convertible bond instrument boilerplate.

pub mod metrics;
pub mod model;

mod builder;
mod types;

pub use types::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};

// Provide a distinct path for types.rs to reference this builder
#[allow(unused_imports)]
pub(crate) mod mod_convertible {
    pub use super::builder::ConvertibleBondBuilder;
}

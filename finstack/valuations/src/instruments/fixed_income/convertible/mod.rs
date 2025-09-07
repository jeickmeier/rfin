//! Convertible bond instrument boilerplate.

pub mod metrics;
pub mod model;

mod types;
mod builder;

pub use types::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};

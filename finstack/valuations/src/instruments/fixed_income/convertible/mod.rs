//! Convertible bond instrument boilerplate.

pub mod metrics;
pub mod model;

mod builder;
mod types;

pub use types::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};

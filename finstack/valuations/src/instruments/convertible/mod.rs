//! Convertible bond instrument boilerplate.

pub mod metrics;
pub mod model;

mod types;

pub use types::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};

// Builder provided by FinancialBuilder derive

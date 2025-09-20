//! CDS pricing facade and engine re-export.
//!
//! Exposes the pricing entrypoints for `CreditDefaultSwap`. Core pricing
//! logic lives in `engine`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.

pub mod engine;

pub use engine::{isda_constants, CDSBootstrapper, CDSPricer, CDSPricerConfig, IntegrationMethod};

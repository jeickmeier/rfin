//! WASM bindings for the `finstack-valuations` crate.
//!
//! Split by domain:
//! - [`pricing`] — instrument JSON validation, pricing, metric introspection.
//! - [`attribution`] — P&L attribution across multiple methodologies.
//! - [`factor_model`] — factor-model sensitivities and risk decomposition.
//! - [`calibration`] — plan-driven calibration engine.
//! - [`correlation`] — mirrors `finstack_valuations::correlation`.

pub mod attribution;
pub mod calibration;
pub mod correlation;
pub mod factor_model;
pub mod pricing;

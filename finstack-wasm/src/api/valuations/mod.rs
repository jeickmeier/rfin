//! WASM bindings for the `finstack-valuations` crate.
//!
//! Split by domain:
//! - [`pricing`] — instrument JSON validation, pricing, metric introspection.
//! - [`analytic`] — closed-form option primitives (Black-Scholes, Black-76, IV).
//! - [`attribution`] — P&L attribution across multiple methodologies.
//! - [`factor_model`] — factor-model sensitivities and risk decomposition.
//! - [`calibration`] — plan-driven calibration engine.
//! - [`correlation`] — mirrors `finstack_valuations::correlation`.
//! - [`fourier`] — COS-method Fourier pricers (Black-Scholes, VG, Merton).
//! - [`exotic_rates`] — deterministic TARN / snowball / range-accrual helpers.
//! - [`sabr`] — SABR parameters, model, smile, and calibrator.

pub mod analytic;
pub mod attribution;
pub mod calibration;
pub mod correlation;
pub mod exotic_rates;
pub mod factor_model;
pub mod fourier;
pub mod fx;
pub mod pricing;
pub mod sabr;

//! Quantitative models for derivatives pricing and risk management.
//!
//! This module contains advanced pricing models including:
//! - SABR volatility model for smile dynamics
//! - Other stochastic volatility models (future)
//! - Local volatility models (future)

pub mod sabr;

pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};

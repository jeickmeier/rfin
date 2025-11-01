//! Volatility models and Black-Scholes helpers.
//!
//! This module provides volatility models (SABR) and helper functions
//! for Black-Scholes calculations.

pub mod black;
pub mod sabr;

pub use black::{d1, d1_black76, d2, d2_black76};
pub use finstack_core::math::{norm_cdf, norm_pdf};
pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};


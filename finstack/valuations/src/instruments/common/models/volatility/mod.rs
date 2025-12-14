//! Volatility models and Black-Scholes helpers.
//!
//! This module provides volatility models (SABR) and helper functions
//! for Black-Scholes calculations.

pub mod black;
pub mod heston;
pub mod local_vol;
pub mod normal;
pub mod sabr;
pub mod sabr_derivatives;

pub use black::{d1, d1_black76, d2, d2_black76};
pub use finstack_core::math::{norm_cdf, norm_pdf};
pub use heston::{HestonModel, HestonParameters};
pub use local_vol::{LocalVolBuilder, LocalVolSurface};
pub use normal::{bachelier_price, d_bachelier};
pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};
pub use sabr_derivatives::{SABRCalibrationDerivatives, SABRMarketData};

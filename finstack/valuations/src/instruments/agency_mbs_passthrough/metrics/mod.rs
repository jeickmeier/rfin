//! Agency MBS risk metrics.
//!
//! This module provides MBS-specific risk metrics including:
//!
//! - **OAS (Option-Adjusted Spread)**: Spread over risk-free that equates
//!   model price to market price
//! - **Effective Duration**: Duration accounting for prepayment sensitivity
//! - **Effective Convexity**: Convexity accounting for prepayment sensitivity
//! - **Key-Rate DV01**: Bucketed interest rate sensitivities

pub mod duration;
pub mod key_rate;
pub mod oas;

pub use duration::{effective_convexity, effective_duration};
pub use key_rate::key_rate_dv01;
pub use oas::calculate_oas;

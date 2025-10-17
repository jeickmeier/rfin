//! Metrics helpers shared across instruments.
//!
//! Keep formulas centralized to reduce duplication and ensure market-standard
//! implementations across instruments.

use crate::constants::ONE_BASIS_POINT;

/// Compute DV01 from present value and modified duration.
///
/// Market-standard: DV01 = Price × ModifiedDuration × 1bp
/// - `price_amount` is a currency amount (e.g., dirty PV)
/// - `modified_duration` is dimensionless (years)
#[inline]
pub fn dv01_from_modified_duration(price_amount: f64, modified_duration: f64) -> f64 {
    if price_amount == 0.0 || modified_duration == 0.0 {
        return 0.0;
    }
    price_amount * modified_duration * ONE_BASIS_POINT
}



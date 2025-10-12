//! Basket metrics module.
//!
//! Provides metric calculators specific to `Basket`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_basket_metrics`.
//!
//! Exposed metrics:
//! - Constituent count
//! - Expense ratio (percentage)
//! - Asset exposure by `AssetType`
//!
//! Note: Present value is handled by the instrument's built-in value() method.

mod asset_exposure;
mod constituent_count;
mod expense_ratio;

use crate::metrics::MetricRegistry;

pub use asset_exposure::AssetExposureCalculator;
pub use constituent_count::ConstituentCountCalculator;
pub use expense_ratio::ExpenseRatioCalculator;

/// Register all Basket metrics with the registry
pub fn register_basket_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Basket",
        metrics: [
            (ConstituentCount, ConstituentCountCalculator),
            (ExpenseRatio, ExpenseRatioCalculator),
        ]
    };
}

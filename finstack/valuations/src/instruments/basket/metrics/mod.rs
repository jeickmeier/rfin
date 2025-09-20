//! Basket metrics module.
//!
//! Provides metric calculators specific to `Basket`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_basket_metrics`.
//!
//! Exposed metrics:
//! - Net Asset Value per share
//! - Basket total value
//! - Constituent count
//! - Expense ratio (percentage)
//! - Tracking error (placeholder without history)
//! - Utilization vs creation unit size
//! - Premium/Discount vs market price
//! - Asset exposure by `AssetType`

mod asset_exposure;
mod basket_value;
mod constituent_count;
mod expense_ratio;
mod nav;
mod premium_discount;
mod tracking_error;
mod utilization;

use crate::metrics::MetricRegistry;

pub use asset_exposure::AssetExposureCalculator;
pub use basket_value::BasketValueCalculator;
pub use constituent_count::ConstituentCountCalculator;
pub use expense_ratio::ExpenseRatioCalculator;
pub use nav::NavCalculator;
pub use premium_discount::PremiumDiscountCalculator;
pub use tracking_error::TrackingErrorCalculator;
pub use utilization::UtilizationCalculator;

/// Register all Basket metrics with the registry
pub fn register_basket_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(MetricId::Nav, Arc::new(NavCalculator), &["Basket"])
        .register_metric(
            MetricId::BasketValue,
            Arc::new(BasketValueCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::ConstituentCount,
            Arc::new(ConstituentCountCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::ExpenseRatio,
            Arc::new(ExpenseRatioCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::TrackingError,
            Arc::new(TrackingErrorCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::Utilization,
            Arc::new(UtilizationCalculator),
            &["Basket"],
        )
        .register_metric(
            MetricId::PremiumDiscount,
            Arc::new(PremiumDiscountCalculator),
            &["Basket"],
        );
}

//! ABS-specific metric calculators.
//!
//! Implements market-standard metrics for Asset-Backed Securities:
//! - ABS Speed (for auto loans)
//! - MPR (Monthly Payment Rate for credit cards)
//! - Delinquency Rate
//! - Charge-Off Rate
//! - WAL (Weighted Average Life)
//! - Credit Enhancement Level
//! - Excess Spread
//! - Average Life

mod abs_speed;
mod delinquency;
mod excess_spread;

pub use abs_speed::AbsSpeedCalculator;
pub use delinquency::{AbsChargeOffCalculator, AbsDelinquencyCalculator};
pub use excess_spread::{AbsCreditEnhancementCalculator, AbsExcessSpreadCalculator};

use crate::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all ABS metrics
pub fn register_abs_metrics(registry: &mut MetricRegistry) {
    // ABS Speed (for auto loans)
    registry.register_metric(
        MetricId::custom("abs_speed"),
        Arc::new(AbsSpeedCalculator),
        &["ABS"],
    );

    // Delinquency Rate
    registry.register_metric(
        MetricId::custom("abs_delinquency"),
        Arc::new(AbsDelinquencyCalculator),
        &["ABS"],
    );

    // Charge-Off Rate
    registry.register_metric(
        MetricId::custom("abs_charge_off"),
        Arc::new(AbsChargeOffCalculator),
        &["ABS"],
    );

    // Excess Spread
    registry.register_metric(
        MetricId::custom("abs_excess_spread"),
        Arc::new(AbsExcessSpreadCalculator),
        &["ABS"],
    );

    // Credit Enhancement Level
    registry.register_metric(
        MetricId::custom("abs_ce_level"),
        Arc::new(AbsCreditEnhancementCalculator),
        &["ABS"],
    );

    // WAL
    registry.register_metric(
        MetricId::custom("abs_wal"),
        Arc::new(AbsWalCalculator),
        &["ABS"],
    );
}

/// WAL Calculator for ABS
struct AbsWalCalculator;

impl crate::metrics::MetricCalculator for AbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::abs::Abs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        Ok(abs.pool.weighted_avg_maturity(context.as_of))
    }
}

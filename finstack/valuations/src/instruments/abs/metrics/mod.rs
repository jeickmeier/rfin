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

use crate::metrics::{MetricContext, MetricRegistry};

/// Register all ABS metrics
pub fn register_abs_metrics(registry: &mut MetricRegistry) {
    use crate::instruments::common::structured_credit::metrics as sc;
    
    crate::register_metrics! {
        registry: registry,
        instrument: "ABS",
        metrics: [
            // ABS-specific metrics
            (CPR, AbsSpeedCalculator),  // ABS prepayment speed (similar concept to CPR)
            (AbsDelinquency, AbsDelinquencyCalculator),
            (AbsChargeOff, AbsChargeOffCalculator),
            (AbsExcessSpread, AbsExcessSpreadCalculator),
            (AbsCreditEnhancement, AbsCreditEnhancementCalculator),
            // Shared structured credit metrics
            (WAL, AbsWalCalculator),
            (Accrued, sc::AccruedCalculator),
            (DirtyPrice, sc::DirtyPriceCalculator),
            (CleanPrice, sc::CleanPriceCalculator),
            (DurationMac, sc::MacaulayDurationCalculator),
            (DurationMod, sc::ModifiedDurationCalculator),
            (ZSpread, sc::ZSpreadCalculator),
            (Cs01, sc::Cs01Calculator),
            (SpreadDuration, sc::SpreadDurationCalculator),
            (Ytm, sc::YtmCalculator),
            (WAM, sc::WamCalculator),
            (CPR, sc::CprCalculator),
            (CDR, sc::CdrCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01::<
                crate::instruments::Abs,
            >::default()),
        ]
    }
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

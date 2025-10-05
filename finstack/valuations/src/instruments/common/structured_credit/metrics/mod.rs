//! Metric calculators for structured credit instruments (CLO, ABS, RMBS, CMBS).
//!
//! This module provides market-standard metrics for structured credit, including:
//! - Accrued interest
//! - Clean/dirty prices
//! - WAL (Weighted Average Life)
//! - Durations (Macaulay, Modified)
//! - Z-spread (consistent across all instruments)
//! - CS01 and spread duration (spread risk)

pub mod accrued;
pub mod duration;
pub mod pool;
pub mod prices;
pub mod spreads;
pub mod wal;
pub mod ytm;

pub use accrued::AccruedCalculator;
pub use duration::{MacaulayDurationCalculator, ModifiedDurationCalculator};
pub use pool::{CdrCalculator, CprCalculator, WamCalculator};
pub use prices::{CleanPriceCalculator, DirtyPriceCalculator};
pub use spreads::{Cs01Calculator, SpreadDurationCalculator, ZSpreadCalculator};
pub use wal::WalCalculator;
pub use ytm::YtmCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all structured credit metrics to a registry.
pub fn register_structured_credit_metrics(registry: &mut MetricRegistry) {
    let sc_types = &["CLO", "ABS", "RMBS", "CMBS"];
    
    // Accrued interest
    registry.register_metric(
        MetricId::Accrued,
        Arc::new(AccruedCalculator),
        sc_types,
    );
    
    // Prices
    registry.register_metric(
        MetricId::DirtyPrice,
        Arc::new(DirtyPriceCalculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::CleanPrice,
        Arc::new(CleanPriceCalculator),
        sc_types,
    );
    
    // WAL (Weighted Average Life)
    registry.register_metric(
        MetricId::WAL,
        Arc::new(WalCalculator),
        sc_types,
    );
    
    // Durations
    registry.register_metric(
        MetricId::DurationMac,
        Arc::new(MacaulayDurationCalculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::DurationMod,
        Arc::new(ModifiedDurationCalculator),
        sc_types,
    );
    
    // Spread Metrics & Risk
    registry.register_metric(
        MetricId::ZSpread,
        Arc::new(ZSpreadCalculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::Cs01,
        Arc::new(Cs01Calculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::SpreadDuration,
        Arc::new(SpreadDurationCalculator),
        sc_types,
    );
    
    // YTM (Yield to Maturity)
    registry.register_metric(
        MetricId::Ytm,
        Arc::new(YtmCalculator),
        sc_types,
    );
    
    // Pool Characteristic Metrics
    registry.register_metric(
        MetricId::WAM,
        Arc::new(WamCalculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::CPR,
        Arc::new(CprCalculator),
        sc_types,
    );
    
    registry.register_metric(
        MetricId::CDR,
        Arc::new(CdrCalculator),
        sc_types,
    );
}


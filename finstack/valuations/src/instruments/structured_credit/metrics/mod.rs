//! Consolidated metrics for all structured credit instruments.
//!
//! This module contains metrics from ABS, CLO, CMBS, and RMBS.
//! Some metrics are deal-type specific and will only compute for appropriate instruments.

// ABS-specific metrics
pub mod abs_speed;
pub mod delinquency;
pub mod excess_spread;

// CLO-specific metrics
pub mod warf;
pub mod was;

// CMBS-specific metrics
pub mod dscr;

// Shared metrics (renamed temporarily to avoid conflicts)
pub mod ltv_cmbs;
pub mod ltv_rmbs;
pub mod wal_clo;
pub mod wal_rmbs;

// Re-exports for convenience
pub use abs_speed::*;
pub use delinquency::*;
pub use dscr::*;
pub use excess_spread::*;
pub use ltv_cmbs::*;
pub use ltv_rmbs::*;
pub use wal_clo::*;
pub use wal_rmbs::*;
pub use warf::*;
pub use was::*;

/// Register all structured credit metrics
pub fn register_structured_credit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::instruments::common::structured_credit::metrics::{
        AccruedCalculator, CdrCalculator, CleanPriceCalculator, CprCalculator, Cs01Calculator,
        DirtyPriceCalculator, MacaulayDurationCalculator, ModifiedDurationCalculator,
        SpreadDurationCalculator, WalCalculator, WamCalculator, YtmCalculator, ZSpreadCalculator,
    };
    
    crate::register_metrics! {
        registry: registry,
        instrument: "StructuredCredit",
        metrics: [
            // Standard cashflow-based metrics
            (Accrued, AccruedCalculator),
            (DirtyPrice, DirtyPriceCalculator),
            (CleanPrice, CleanPriceCalculator),
            (WAL, WalCalculator),
            (DurationMac, MacaulayDurationCalculator),
            (DurationMod, ModifiedDurationCalculator),
            (Ytm, YtmCalculator),
            (ZSpread, ZSpreadCalculator),
            (Cs01, Cs01Calculator),
            (SpreadDuration, SpreadDurationCalculator),
            // Pool metrics
            (WAM, WamCalculator),
            (CPR, CprCalculator),
            (CDR, CdrCalculator)
        ]
    }
    
    // Note: WARF and WAS would need custom MetricId variants to be registered
    // Other metrics (ABS speed, delinquency, etc.) also need custom MetricId variants
}

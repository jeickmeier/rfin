//! Metrics for structured credit instruments.
//!
//! This module organizes metrics by category:
//! - pricing: Valuation-focused metrics (prices, accrued, WAL)
//! - risk: Risk and sensitivity metrics (duration, spreads, YTM)
//! - pool: Collateral pool characteristics (WAM, CPR, CDR, WARF, WAS)
//! - deal_specific: Deal-type specific metrics (ABS, CLO, CMBS, RMBS)

pub mod pricing;
pub mod risk;
pub mod pool;
pub mod deal_specific;

// Re-export all calculators for convenience
pub use pricing::*;
pub use risk::*;
pub use pool::*;
pub use deal_specific::*;

/// Register all structured credit metrics
pub fn register_structured_credit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "StructuredCredit",
        metrics: [
            // Standard cashflow-based metrics
            (Accrued, pricing::AccruedCalculator),
            (DirtyPrice, pricing::DirtyPriceCalculator),
            (CleanPrice, pricing::CleanPriceCalculator),
            (WAL, pricing::WalCalculator),
            (DurationMac, risk::MacaulayDurationCalculator),
            (DurationMod, risk::ModifiedDurationCalculator),
            (Ytm, risk::YtmCalculator),
            (ZSpread, risk::ZSpreadCalculator),
            (Cs01, risk::Cs01Calculator),
            (SpreadDuration, risk::SpreadDurationCalculator),
            // Pool metrics
            (WAM, pool::WamCalculator),
            (CPR, pool::CprCalculator),
            (CDR, pool::CdrCalculator)
        ]
    }
    
    // Note: Deal-specific metrics (WARF, WAS, ABS speed, delinquency, DSCR, excess spread,
    // LTV, FICO) would need custom MetricId variants in the core metrics module to be registered.
    // These are currently used directly via their calculator structs when needed.
}

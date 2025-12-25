//! Metrics for structured credit instruments.
//!
//! This module organizes metrics by category:
//! - pricing: Valuation-focused metrics (prices, accrued, WAL)
//! - risk: Risk and sensitivity metrics (duration, spreads, YTM)
//! - pool: Collateral pool characteristics (WAM, CPR, CDR, WARF, WAS)
//! - deal_specific: Deal-type specific metrics (ABS, CLO, CMBS, RMBS)

pub mod deal_specific;
// pub mod dv01; // removed - using GenericParallelDv01
pub mod pool;
pub mod pricing;
pub mod risk;

// Re-export all calculators for convenience
pub use deal_specific::*;
// pub use dv01::StructuredCreditDv01Calculator; // removed - using GenericParallelDv01
pub use pool::*;
pub use pricing::*;
pub use risk::*;

// Re-export standalone tranche metric functions for backward compatibility
pub use pricing::calculate_tranche_wal;
pub use risk::{calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_z_spread};

/// Register all structured credit metrics
pub fn register_structured_credit_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Model-specific risk metrics (custom metrics)
    registry.register_metric(
        MetricId::Recovery01,
        Arc::new(risk::recovery01::Recovery01Calculator),
        &[InstrumentType::StructuredCredit],
    );
    registry.register_metric(
        MetricId::Prepayment01,
        Arc::new(risk::prepayment01::Prepayment01Calculator),
        &[InstrumentType::StructuredCredit],
    );
    registry.register_metric(
        MetricId::Default01,
        Arc::new(risk::default01::Default01Calculator),
        &[InstrumentType::StructuredCredit],
    );
    registry.register_metric(
        MetricId::Severity01,
        Arc::new(risk::severity01::Severity01Calculator),
        &[InstrumentType::StructuredCredit],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::StructuredCredit,
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
            // Note: BucketedCs01 not registered - StructuredCredit uses pool-based credit models
            // (CDR/default rates) rather than a credit curve
            (SpreadDuration, risk::SpreadDurationCalculator),
            // Pool metrics
            (WAM, pool::WamCalculator),
            (CPR, pool::CprCalculator),
            (CDR, pool::CdrCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::structured_credit::StructuredCredit,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::structured_credit::StructuredCredit,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            // Theta is now registered universally in metrics::standard_registry()
        ]
    }

    // Note: Deal-specific metrics (WARF, WAS, ABS speed, delinquency, DSCR, excess spread,
    // LTV, FICO) would need custom MetricId variants in the core metrics module to be registered.
    // These are currently used directly via their calculator structs when needed.
}

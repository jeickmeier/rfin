//! Basis swap specific metrics.
//!
//! Provides calculators for leg present values, discounted accrual sums (annuities),
//! DV01s, and par spread calculations for basis swap instruments.

pub mod annuity;
pub mod dv01;
pub mod par_spread;
pub mod pv;
pub mod risk_bucketed_dv01;

pub use annuity::AnnuityCalculator;
pub use dv01::Dv01Calculator;
pub use par_spread::ParSpreadCalculator;
pub use pv::PvCalculator;
pub use risk_bucketed_dv01::BucketedDv01Calculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Registers all basis swap metrics in the standard metric registry.
///
/// This function adds the basis swap specific metric calculators to the provided
/// registry, making them available for use in valuation calculations.
///
/// # Arguments
/// * `registry` — The metric registry to register the calculators with
pub fn register_basis_swap_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::BasisAnnuityPrimary,
            Arc::new(AnnuityCalculator::primary()),
            &["BasisSwap"],
        ) // discounted accrual sum
        .register_metric(
            MetricId::BasisAnnuityReference,
            Arc::new(AnnuityCalculator::reference()),
            &["BasisSwap"],
        ) // discounted accrual sum
        .register_metric(
            MetricId::BasisDv01Primary,
            Arc::new(Dv01Calculator::primary()),
            &["BasisSwap"],
        ) // 1bp DV01
        .register_metric(
            MetricId::BasisDv01Reference,
            Arc::new(Dv01Calculator::reference()),
            &["BasisSwap"],
        ) // 1bp DV01
        .register_metric(
            MetricId::BasisPvPrimary,
            Arc::new(PvCalculator::primary()),
            &["BasisSwap"],
        ) // leg PV
        .register_metric(
            MetricId::BasisPvReference,
            Arc::new(PvCalculator::reference()),
            &["BasisSwap"],
        ) // leg PV
        .register_metric(
            MetricId::BasisParSpread,
            Arc::new(ParSpreadCalculator),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(BucketedDv01Calculator),
            &["BasisSwap"],
        );
}

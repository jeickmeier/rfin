//! Basis swap specific metrics.
//!
//! Provides calculators for leg present values, discounted accrual sums (annuities),
//! DV01s, and par spread calculations for basis swap instruments.

pub mod annuity;
pub mod dv01;
pub mod net_dv01;
pub mod par_spread;
pub mod pv;
// risk_bucketed_dv01 and theta now using generic implementations

pub use annuity::AnnuityCalculator;
pub use dv01::Dv01Calculator;
pub use net_dv01::NetDv01Calculator;
pub use par_spread::ParSpreadCalculator;
pub use pv::PvCalculator;
// BucketedDv01Calculator now using generic implementation

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
    // Leg-specific metrics with primary/reference constructors (consistent with IRS)
    registry
        .register_metric(
            MetricId::AnnuityPrimary,
            Arc::new(AnnuityCalculator::primary()),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::AnnuityReference,
            Arc::new(AnnuityCalculator::reference()),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::Dv01Primary,
            Arc::new(Dv01Calculator::primary()),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::Dv01Reference,
            Arc::new(Dv01Calculator::reference()),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::PvPrimary,
            Arc::new(PvCalculator::primary()),
            &["BasisSwap"],
        )
        .register_metric(
            MetricId::PvReference,
            Arc::new(PvCalculator::reference()),
            &["BasisSwap"],
        );

    // Net metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "BasisSwap",
        metrics: [
            (Dv01, NetDv01Calculator),
            (BasisParSpread, ParSpreadCalculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::BasisSwap,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::BasisSwap,
            >::default()),
        ]
    }
}

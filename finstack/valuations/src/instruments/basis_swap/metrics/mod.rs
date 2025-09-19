//! Basis swap specific metrics.
//!
//! Provides calculators for leg PVs, discounted accrual sums (annuities), DV01s,
//! and par spread on the primary leg.

pub mod annuity;
pub mod dv01;
pub mod pv;
pub mod par_spread;

pub use annuity::AnnuityCalculator;
pub use dv01::Dv01Calculator;
pub use pv::PvCalculator;
pub use par_spread::ParSpreadCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register basis swap metrics in the standard registry.
pub fn register_basis_swap_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(MetricId::BasisAnnuityPrimary, Arc::new(AnnuityCalculator::primary()), &["BasisSwap"]) // discounted accrual sum
        .register_metric(MetricId::BasisAnnuityReference, Arc::new(AnnuityCalculator::reference()), &["BasisSwap"]) // discounted accrual sum
        .register_metric(MetricId::BasisDv01Primary, Arc::new(Dv01Calculator::primary()), &["BasisSwap"]) // 1bp DV01
        .register_metric(MetricId::BasisDv01Reference, Arc::new(Dv01Calculator::reference()), &["BasisSwap"]) // 1bp DV01
        .register_metric(MetricId::BasisPvPrimary, Arc::new(PvCalculator::primary()), &["BasisSwap"]) // leg PV
        .register_metric(MetricId::BasisPvReference, Arc::new(PvCalculator::reference()), &["BasisSwap"]) // leg PV
        .register_metric(MetricId::BasisParSpread, Arc::new(ParSpreadCalculator), &["BasisSwap"]);
}


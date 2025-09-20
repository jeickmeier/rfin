//! CDS Index hazard-bump CS01 metric calculator.
//!
//! Computes PV sensitivity to a parallel additive bump in hazard rates of 1bp
//! across all relevant hazard curves.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpUnits};
use finstack_core::Result;
use crate::instruments::traits::Priceable;

/// Hazard CS01 calculator for CDS Index (parallel hazard bump)
pub struct HazardCs01Calculator;

impl MetricCalculator for HazardCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;

        // Base PV
        let base = idx.value(&context.curves, context.as_of)?;

        // Build a +1bp hazard bump across all hazard curves in the context
        // Note: reuse MarketContext::bump infrastructure; here we only bump curves
        // used by the index paths implicitly through value() recomputation.
        let mut bumps = hashbrown::HashMap::new();
        // Conservative approach: bump known hazard curves used by instrument
        // Premium leg uses `cds.premium.disc_id`; for hazard we rely on constituents
        // or index hazard id present in the instrument fields or credit params.
        // Here we bump ALL hazard curves present in MarketContext by probing ids.
        // We cannot access private fields; iterate over curve_ids and try get_ref::<HazardCurve>.
        for cid in context.curves.curve_ids() {
            if context.curves.get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(cid.as_str()).is_ok() {
                bumps.insert(cid.clone(), BumpSpec { mode: BumpMode::Additive, units: BumpUnits::RateBp, value: 1.0 });
            }
        }
        let bumped_ctx = context.curves.bump(bumps)?;
        let bumped = idx.value(&bumped_ctx, context.as_of)?;

        Ok((bumped.amount() - base.amount()).abs())
    }
}



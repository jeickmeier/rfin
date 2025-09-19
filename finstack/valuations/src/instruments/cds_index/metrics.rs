//! CDS Index metrics
//!
//! Reuse the single-name CDS calculators by delegating to a synthetic CDS
//! constructed from the index fields.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Par spread calculator for CDS Index
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let cds = idx.to_synthetic_cds();
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id,
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id,
        )?;
        cds.par_spread(disc, surv)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Risky PV01 calculator for CDS Index
pub struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let cds = idx.to_synthetic_cds();
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id,
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id,
        )?;
        cds.risky_pv01(disc, surv)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// CS01 calculator for CDS Index
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let cds = idx.to_synthetic_cds();
        let pricer = crate::instruments::cds::cds_pricer::CDSPricer::new();
        pricer.cs01(&cds, &context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Protection leg PV calculator
pub struct ProtectionLegPvCalculator;

impl MetricCalculator for ProtectionLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let cds = idx.to_synthetic_cds();
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id,
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id,
        )?;
        let pv = cds.pv_protection_leg(disc, surv)?;
        Ok(pv.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Premium leg PV calculator
pub struct PremiumLegPvCalculator;

impl MetricCalculator for PremiumLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let cds = idx.to_synthetic_cds();
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            cds.premium.disc_id,
        )?;
        let surv = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            cds.protection.credit_id,
        )?;
        let pv = cds.pv_premium_leg(disc, surv)?;
        Ok(pv.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register all CDS Index metrics with the registry
pub fn register_cds_index_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::ParSpread,
        Arc::new(ParSpreadCalculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::RiskyPv01,
        Arc::new(RiskyPv01Calculator),
        &["CDSIndex"],
    );
    registry.register_metric(MetricId::Cs01, Arc::new(Cs01Calculator), &["CDSIndex"]);
    registry.register_metric(
        MetricId::ProtectionLegPv,
        Arc::new(ProtectionLegPvCalculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::PremiumLegPv,
        Arc::new(PremiumLegPvCalculator),
        &["CDSIndex"],
    );
}

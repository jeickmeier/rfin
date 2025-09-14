//! CDS-specific metrics calculators

use crate::instruments::fixed_income::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Par spread calculator for CDS
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.discount(cds.premium.disc_id)?;
        let surv = context.curves.hazard(cds.protection.credit_id)?;
        cds.par_spread(&*disc, surv.as_ref())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Risky PV01 calculator for CDS
pub struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.discount(cds.premium.disc_id)?;
        let surv = context.curves.hazard(cds.protection.credit_id)?;
        cds.risky_pv01(&*disc, surv.as_ref())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// CS01 calculator for CDS
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        cds.cs01(&context.curves)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Protection leg PV calculator
pub struct ProtectionLegPvCalculator;

impl MetricCalculator for ProtectionLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.discount(cds.premium.disc_id)?;
        let surv = context.curves.hazard(cds.protection.credit_id)?;
        let pv = cds.pv_protection_leg(&*disc, surv.as_ref())?;
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
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let disc = context.curves.discount(cds.premium.disc_id)?;
        let surv = context.curves.hazard(cds.protection.credit_id)?;
        let pv = cds.pv_premium_leg(&*disc, surv.as_ref())?;
        Ok(pv.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register all CDS metrics with the registry
pub fn register_cds_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(MetricId::ParSpread, Arc::new(ParSpreadCalculator), &["CDS"]);

    registry.register_metric(MetricId::RiskyPv01, Arc::new(RiskyPv01Calculator), &["CDS"]);

    registry.register_metric(MetricId::Cs01, Arc::new(Cs01Calculator), &["CDS"]);

    registry.register_metric(
        MetricId::ProtectionLegPv,
        Arc::new(ProtectionLegPvCalculator),
        &["CDS"],
    );

    registry.register_metric(
        MetricId::PremiumLegPv,
        Arc::new(PremiumLegPvCalculator),
        &["CDS"],
    );
}

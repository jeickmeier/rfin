//! ILB-specific metrics calculators

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Real yield calculator for ILB
pub struct RealYieldCalculator;

impl MetricCalculator for RealYieldCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        let clean_price = ilb.quoted_clean.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "inflation_linked_bond_quote".to_string(),
            })
        })?;
        ilb.real_yield(clean_price, &context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Index ratio calculator for ILB
pub struct IndexRatioCalculator;

impl MetricCalculator for IndexRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        // Get inflation index
        let inflation_index = context
            .curves
            .inflation_index(ilb.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_linked_bond_quote".to_string(),
                })
            })?;

        ilb.index_ratio(context.as_of, &inflation_index)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Real duration calculator for ILB
pub struct RealDurationCalculator;

impl MetricCalculator for RealDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        ilb.real_duration(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Breakeven inflation calculator for ILB
pub struct BreakevenInflationCalculator;

impl MetricCalculator for BreakevenInflationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _ilb: &InflationLinkedBond = context.instrument_as()?;
        // Breakeven inflation requires a nominal bond yield which is not available
        // in the current market context. This metric should be computed externally
        // with the appropriate nominal yield input.
        Err(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "inflation_linked_bond_quote".to_string(),
            },
        ))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // Would need static storage for custom MetricId
    }
}

/// Register all ILB metrics with the registry
pub fn register_ilb_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::custom("real_yield"),
        Arc::new(RealYieldCalculator),
        &["ILB"],
    );

    registry.register_metric(
        MetricId::custom("index_ratio"),
        Arc::new(IndexRatioCalculator),
        &["ILB"],
    );

    registry.register_metric(
        MetricId::custom("real_duration"),
        Arc::new(RealDurationCalculator),
        &["ILB"],
    );

    registry.register_metric(
        MetricId::custom("breakeven_inflation"),
        Arc::new(BreakevenInflationCalculator),
        &["ILB"],
    );
}

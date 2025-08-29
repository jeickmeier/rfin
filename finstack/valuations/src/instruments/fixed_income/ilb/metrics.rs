//! ILB-specific metrics calculators

use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{F, Result};
use std::sync::Arc;

/// Real yield calculator for ILB
pub struct RealYieldCalculator;

impl MetricCalculator for RealYieldCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;
        
        if let Instrument::ILB(ilb) = &*context.instrument {
            ilb.real_yield(
                ilb.quoted_clean.unwrap_or(100.0),
                &context.curves,
                context.as_of,
            )
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))
        }
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Index ratio calculator for ILB
pub struct IndexRatioCalculator;

impl MetricCalculator for IndexRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;
        
        if let Instrument::ILB(ilb) = &*context.instrument {
            // Get inflation index
            let inflation_index = context.curves.inflation_index(ilb.inflation_id)
                .ok_or_else(|| finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound
                ))?;
            
            ilb.index_ratio(context.as_of, &inflation_index)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))
        }
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Real duration calculator for ILB
pub struct RealDurationCalculator;

impl MetricCalculator for RealDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;
        
        if let Instrument::ILB(ilb) = &*context.instrument {
            ilb.real_duration(&context.curves, context.as_of)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))
        }
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Breakeven inflation calculator for ILB
pub struct BreakevenInflationCalculator;

impl MetricCalculator for BreakevenInflationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        use crate::instruments::Instrument;
        
        if let Instrument::ILB(ilb) = &*context.instrument {
            // Would need nominal bond yield from market context
            // For now, use a placeholder
            let nominal_yield = 0.03; // 3% nominal yield
            ilb.breakeven_inflation(nominal_yield, &context.curves, context.as_of)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))
        }
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

//! Price calculators for structured credit instruments.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Calculates dirty price as percentage of par (includes accrued interest).
///
/// Dirty price is the market value including accrued interest, expressed as
/// a percentage of the original notional. This is the actual transaction price.
///
/// # Formula
///
/// Dirty Price = (NPV / Original Notional) × 100
///
/// Where NPV is the net present value of all future cashflows.
///
pub struct DirtyPriceCalculator;

impl MetricCalculator for DirtyPriceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Use the base NPV that was already computed
        let npv = context.base_value.amount();
        
        // Get the original notional
        // For structured credit, this is typically stored in the pool or tranche
        let notional = get_original_notional(context)?;
        
        if notional == 0.0 {
            return Ok(0.0);
        }
        
        // Dirty price = (NPV / Notional) × 100
        let dirty_price = (npv / notional) * 100.0;
        
        Ok(dirty_price)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[] // Uses base NPV from context
    }
}

/// Calculates clean price as percentage of par (excludes accrued interest).
///
/// Clean price is the market convention for quoting structured credit instruments.
/// It equals the dirty price minus accrued interest (converted to price points).
///
/// # Formula
///
/// Clean Price = Dirty Price - (Accrued / Notional) × 100
///
pub struct CleanPriceCalculator;

impl MetricCalculator for CleanPriceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get dirty price from computed metrics
        let dirty = context
            .computed
            .get(&MetricId::DirtyPrice)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:DirtyPrice".to_string(),
                })
            })?;
        
        // Get accrued interest in currency units
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;
        
        // Convert accrued to price points
        let notional = get_original_notional(context)?;
        let accrued_points = if notional > 0.0 {
            (accrued / notional) * 100.0
        } else {
            0.0
        };
        
        // Clean price = Dirty price - Accrued (in points)
        let clean_price = dirty - accrued_points;
        
        Ok(clean_price)
    }
    
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DirtyPrice, MetricId::Accrued]
    }
}

/// Helper to get the original notional from the instrument.
///
/// For structured credit, the notional is typically the tranche size or pool balance.
fn get_original_notional(context: &MetricContext) -> Result<f64> {
    // Try to get notional from the instrument
    // This is a simplified implementation - in practice, you'd check the instrument type
    // and extract the appropriate notional field
    
    use crate::instruments::common::structured_credit::StructuredCreditInstrument;
    
    // Try to downcast to StructuredCreditInstrument
    if let Some(sc_instr) = context.instrument.as_any()
        .downcast_ref::<&dyn StructuredCreditInstrument>()
    {
        // Get total pool balance as notional
        let pool = sc_instr.pool();
        return Ok(pool.total_balance().amount());
    }
    
    // Fallback: try to extract from base_value (should be in currency units)
    // This is a conservative fallback
    Ok(context.base_value.amount().abs())
}


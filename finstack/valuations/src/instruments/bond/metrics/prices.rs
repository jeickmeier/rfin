use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};


/// Calculates dirty price for bonds (clean price + accrued interest).
pub struct DirtyPriceCalculator;

impl MetricCalculator for DirtyPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Dirty price only makes sense if we have a quoted clean price
        let clean_px = bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "bond.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;

        // Get accrued from computed metrics
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Dirty price in currency = (clean % of par) * notional + accrued (currency)
        Ok(clean_px * bond.notional.amount() / 100.0 + accrued)
    }
}

/// Calculates clean price for bonds (dirty price - accrued interest).
pub struct CleanPriceCalculator;

impl MetricCalculator for CleanPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // If we have quoted clean price, return currency value
        if let Some(clean_px) = bond.pricing_overrides.quoted_clean_price {
            return Ok(clean_px * bond.notional.amount() / 100.0);
        }

        // Otherwise calculate from base value (which should be dirty price in currency)
        let dirty_px = context.base_value.amount();
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "metric:Accrued".to_string(),
                })
            })?;

        // Clean price in currency
        Ok(dirty_px - accrued)
    }
}

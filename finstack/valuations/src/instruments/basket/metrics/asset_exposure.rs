//! Asset exposure metric calculator.
//!
//! Computes effective exposure by `AssetType` based on constituent weights.

use crate::instruments::basket::types::{AssetType, Basket, ConstituentReference};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result};

/// Calculate effective exposure by asset type
pub struct AssetExposureCalculator {
    pub(crate) asset_type: AssetType,
}

impl AssetExposureCalculator {
    pub fn new(asset_type: AssetType) -> Self {
        Self { asset_type }
    }
}

impl MetricCalculator for AssetExposureCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let basket = context.instrument_as::<Basket>()?;
        let mut total_exposure = 0.0;
        for constituent in &basket.constituents {
            let matches = match (&constituent.reference, &self.asset_type) {
                (ConstituentReference::MarketData { asset_type, .. }, target) => {
                    std::mem::discriminant(asset_type) == std::mem::discriminant(target)
                }
                (ConstituentReference::Instrument(instrument), target) => {
                    use crate::pricer::InstrumentType;
                    let it = instrument.key();
                    matches!(
                        (it, target),
                        (InstrumentType::Bond, AssetType::Bond)
                            | (InstrumentType::Equity, AssetType::Equity)
                            | (InstrumentType::Basket, AssetType::ETF)
                    )
                }
            };
            if matches {
                total_exposure += constituent.weight;
            }
        }
        Ok(total_exposure * 100.0)
    }
}

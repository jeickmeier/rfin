//! Collateral value metric for `Repo`.
//!
//! Computes the current market value of the collateral backing the repo
//! using prices from `MarketContext`.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculate the market value of collateral.
pub struct CollateralValueCalculator;

impl MetricCalculator for CollateralValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
        let collateral_value = repo.collateral.market_value(&context.curves)?;
        Ok(collateral_value.amount())
    }
}

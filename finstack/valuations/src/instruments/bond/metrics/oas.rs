use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;

/// Calculates Option-Adjusted Spread for bonds with embedded options.
///
/// Uses short-rate trees to value callable/putable bonds and solve for the
/// spread that makes the model price equal to the market price.
pub struct OasCalculator;

impl MetricCalculator for OasCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        // Require quoted clean price
        let clean_price = bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "bond.pricing_overrides.quoted_clean_price".to_string(),
            })
        })?;

        // Use MarketContext directly (no conversion needed)
        let market_context = context.curves.as_ref().clone();

        // Use Tree pricer to solve for OAS
        let oas_calculator = crate::instruments::bond::pricing::tree_pricer::TreePricer::new();
        oas_calculator.calculate_oas(bond, &market_context, context.as_of, clean_price)
    }
}

//! Trait implementations for Autocallable

use crate::instruments::autocallable::Autocallable;
use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;

impl HasEquityUnderlying for Autocallable {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for Autocallable {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

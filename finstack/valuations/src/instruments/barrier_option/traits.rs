//! Trait implementations for BarrierOption

use crate::instruments::barrier_option::BarrierOption;
use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;

impl HasEquityUnderlying for BarrierOption {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for BarrierOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

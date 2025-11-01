//! Trait implementations for LookbackOption

use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::instruments::lookback_option::LookbackOption;

impl HasEquityUnderlying for LookbackOption {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for LookbackOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

//! Trait implementations for AsianOption

use crate::instruments::asian_option::AsianOption;
use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;

impl HasEquityUnderlying for AsianOption {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for AsianOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

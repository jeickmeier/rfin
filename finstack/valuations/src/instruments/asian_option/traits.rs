//! Trait implementations for AsianOption

use crate::instruments::asian_option::AsianOption;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

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

impl HasExpiry for AsianOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

impl HasDayCount for AsianOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

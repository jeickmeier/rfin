//! Trait implementations for LookbackOption

use crate::instruments::lookback_option::LookbackOption;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

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

impl HasExpiry for LookbackOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

impl HasDayCount for LookbackOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

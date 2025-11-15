//! Trait implementations for Autocallable

use crate::instruments::autocallable::Autocallable;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

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

impl HasExpiry for Autocallable {
    fn expiry(&self) -> finstack_core::dates::Date {
        // Autocallable uses final observation date as expiry
        self.observation_dates
            .last()
            .copied()
            .unwrap_or(self.observation_dates[0])
    }
}

impl HasDayCount for Autocallable {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

//! Trait implementations for CliquetOption

use crate::instruments::cliquet_option::CliquetOption;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

impl HasEquityUnderlying for CliquetOption {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for CliquetOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

impl HasExpiry for CliquetOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        // Cliquet uses last reset date as expiry
        self.reset_dates
            .last()
            .copied()
            .unwrap_or(self.reset_dates[0])
    }
}

impl HasDayCount for CliquetOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

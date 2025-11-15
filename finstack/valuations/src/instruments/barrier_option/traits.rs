//! Trait implementations for BarrierOption

use crate::instruments::barrier_option::BarrierOption;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

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

impl HasExpiry for BarrierOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

impl HasDayCount for BarrierOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

//! Trait implementations for RangeAccrual

use crate::instruments::range_accrual::RangeAccrual;
use crate::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry};

impl HasEquityUnderlying for RangeAccrual {
    fn spot_id(&self) -> &str {
        &self.spot_id
    }
}

impl HasPricingOverrides for RangeAccrual {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

impl HasExpiry for RangeAccrual {
    fn expiry(&self) -> finstack_core::dates::Date {
        // RangeAccrual uses last observation date as expiry
        self.observation_dates
            .last()
            .copied()
            .unwrap_or(self.observation_dates[0])
    }
}

impl HasDayCount for RangeAccrual {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

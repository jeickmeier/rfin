//! Trait implementations for RangeAccrual

use crate::instruments::common_impl::traits::EquityDependencies;
use crate::instruments::rates::range_accrual::RangeAccrual;
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};
use finstack_core::dates::Date;

impl EquityDependencies for RangeAccrual {
    fn equity_dependencies(&self) -> crate::instruments::common_impl::traits::EquityInstrumentDeps {
        crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
            .spot(self.spot_id.as_str())
            .vol_surface(self.vol_surface_id.as_str())
            .build()
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
            .or(self.payment_date)
            .unwrap_or(Date::MIN)
    }
}

impl HasDayCount for RangeAccrual {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

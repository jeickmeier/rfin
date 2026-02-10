//! Trait implementations for LookbackOption

use crate::instruments::common_impl::traits::EquityDependencies;
use crate::instruments::exotics::lookback_option::LookbackOption;
#[cfg(feature = "mc")]
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl EquityDependencies for LookbackOption {
    fn equity_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::EquityInstrumentDeps> {
        crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
            .spot(self.spot_id.as_str())
            .vol_surface(self.vol_surface_id.as_str())
            .build()
    }
}

#[cfg(feature = "mc")]
impl HasPricingOverrides for LookbackOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

#[cfg(feature = "mc")]
impl HasExpiry for LookbackOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

#[cfg(feature = "mc")]
impl HasDayCount for LookbackOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

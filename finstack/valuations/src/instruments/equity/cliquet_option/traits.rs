//! Trait implementations for CliquetOption

use crate::instruments::common_impl::traits::EquityDependencies;
use crate::instruments::equity::cliquet_option::CliquetOption;
#[cfg(feature = "mc")]
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl EquityDependencies for CliquetOption {
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
impl HasPricingOverrides for CliquetOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

#[cfg(feature = "mc")]
impl HasExpiry for CliquetOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        // Cliquet uses last reset date as expiry
        self.reset_dates
            .last()
            .copied()
            .unwrap_or(self.reset_dates[0])
    }
}

#[cfg(feature = "mc")]
impl HasDayCount for CliquetOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

//! Trait implementations for CommodityOption.

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::instruments::common_impl::traits::{EquityDependencies, EquityInstrumentDeps};
#[cfg(feature = "mc")]
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl EquityDependencies for CommodityOption {
    fn equity_dependencies(&self) -> finstack_core::Result<EquityInstrumentDeps> {
        let mut builder = EquityInstrumentDeps::builder().vol_surface(self.vol_surface_id.as_str());
        if let Some(ref spot_id) = self.spot_id {
            builder = builder.spot(spot_id.as_str());
        }
        builder.build()
    }
}

#[cfg(feature = "mc")]
impl HasPricingOverrides for CommodityOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

#[cfg(feature = "mc")]
impl HasExpiry for CommodityOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

#[cfg(feature = "mc")]
impl HasDayCount for CommodityOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

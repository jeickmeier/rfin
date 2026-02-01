//! Trait implementations for CommodityOption.

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::instruments::common::traits::{EquityDependencies, EquityInstrumentDeps};
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl EquityDependencies for CommodityOption {
    fn equity_dependencies(&self) -> EquityInstrumentDeps {
        let mut builder = EquityInstrumentDeps::builder().vol_surface(self.vol_surface_id.as_str());
        if let Some(ref spot_id) = self.spot_price_id {
            builder = builder.spot(spot_id.as_str());
        }
        builder.build()
    }
}

impl HasPricingOverrides for CommodityOption {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

impl HasExpiry for CommodityOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.expiry
    }
}

impl HasDayCount for CommodityOption {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

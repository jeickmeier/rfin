//! Trait implementations for LookbackOption

use crate::instruments::common::traits::EquityDependencies;
use crate::instruments::lookback_option::LookbackOption;
use crate::metrics::fd_greeks::{HasDayCount, HasExpiry, HasPricingOverrides};

impl EquityDependencies for LookbackOption {
    fn equity_dependencies(&self) -> crate::instruments::common::traits::EquityInstrumentDeps {
        crate::instruments::common::traits::EquityInstrumentDeps::builder()
            .spot(self.spot_id.as_str())
            .vol_surface(self.vol_surface_id.as_str())
            .build()
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

//! Trait implementations for BarrierOption

use crate::instruments::common_impl::traits::{
    CurveDependencies, EquityDependencies, InstrumentCurves,
};
use crate::instruments::exotics::barrier_option::BarrierOption;
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl CurveDependencies for BarrierOption {
    fn curve_dependencies(&self) -> InstrumentCurves {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl EquityDependencies for BarrierOption {
    fn equity_dependencies(&self) -> crate::instruments::common_impl::traits::EquityInstrumentDeps {
        crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
            .spot(self.spot_id.as_str())
            .vol_surface(self.vol_surface_id.as_str())
            .build()
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

//! Trait implementations for Autocallable

use crate::instruments::common_impl::traits::{
    CurveDependencies, EquityDependencies, InstrumentCurves,
};
use crate::instruments::equity::autocallable::Autocallable;
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl CurveDependencies for Autocallable {
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl EquityDependencies for Autocallable {
    fn equity_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::EquityInstrumentDeps> {
        crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
            .spot(self.spot_id.as_str())
            .vol_surface(self.vol_surface_id.as_str())
            .build()
    }
}

impl HasPricingOverrides for Autocallable {
    fn pricing_overrides_mut(&mut self) -> &mut crate::instruments::PricingOverrides {
        &mut self.pricing_overrides
    }
}

impl HasExpiry for Autocallable {
    fn expiry(&self) -> finstack_core::dates::Date {
        // Autocallable uses final observation date as expiry
        self.observation_dates
            .last()
            .copied()
            .unwrap_or(self.observation_dates[0])
    }
}

impl HasDayCount for Autocallable {
    fn day_count(&self) -> finstack_core::dates::DayCount {
        self.day_count
    }
}

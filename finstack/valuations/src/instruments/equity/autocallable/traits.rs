//! Trait implementations for Autocallable

use crate::instruments::autocallable::Autocallable;
use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{CurveDependencies, EquityDependencies, InstrumentCurves};
use crate::metrics::{HasDayCount, HasExpiry, HasPricingOverrides};

impl CurveDependencies for Autocallable {
    fn curve_dependencies(&self) -> InstrumentCurves {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl HasDiscountCurve for Autocallable {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl EquityDependencies for Autocallable {
    fn equity_dependencies(&self) -> crate::instruments::common::traits::EquityInstrumentDeps {
        crate::instruments::common::traits::EquityInstrumentDeps::builder()
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

//! Trait implementations for RangeAccrual

use crate::instruments::common::metrics::has_equity_underlying::HasEquityUnderlying;
use crate::instruments::common::metrics::has_pricing_overrides::HasPricingOverrides;
use crate::instruments::range_accrual::RangeAccrual;

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


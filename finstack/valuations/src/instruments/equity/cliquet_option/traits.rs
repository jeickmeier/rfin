//! Trait implementations for CliquetOption

use crate::instruments::equity::cliquet_option::CliquetOption;
use crate::metrics::HasExpiry;

crate::impl_equity_exotic_traits!(@equity CliquetOption);
crate::impl_equity_exotic_traits!(@mc_overrides CliquetOption);
crate::impl_equity_exotic_traits!(@mc_daycount CliquetOption);

impl HasExpiry for CliquetOption {
    fn expiry(&self) -> finstack_core::dates::Date {
        self.reset_dates
            .last()
            .copied()
            .unwrap_or(self.reset_dates[0])
    }
}

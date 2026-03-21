//! XVA compatibility layer over `finstack-margin`.
//!
//! Most of the implementation now lives in `finstack-margin`. This module keeps
//! the historical `finstack-valuations::xva::*` surface working, with a small
//! local bridge where valuations still accepts `Arc<dyn Instrument>`.

pub use finstack_margin::xva::{cva, netting, traits, types, Valuable};
pub use netting::*;
pub use types::*;

pub mod exposure {
    //! Exposure-profile compatibility wrappers.

    use std::sync::Arc;

    use crate::instruments::DynInstrument;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_margin::xva::types::{ExposureProfile, NettingSet, XvaConfig};

    use super::bridge::wrap_instruments;

    /// Preserve the historical valuations-side exposure API while the
    /// standalone margin crate transitions callers onto `Valuable`.
    pub fn compute_exposure_profile(
        instruments: &[Arc<DynInstrument>],
        market: &MarketContext,
        as_of: Date,
        config: &XvaConfig,
        netting_set: &NettingSet,
    ) -> finstack_core::Result<ExposureProfile> {
        let valuables = wrap_instruments(instruments);
        finstack_margin::xva::exposure::compute_exposure_profile(
            &valuables,
            market,
            as_of,
            config,
            netting_set,
        )
    }

    #[cfg(feature = "mc")]
    pub use finstack_margin::xva::exposure::compute_stochastic_exposure_profile;
}

pub use cva::*;
pub use exposure::compute_exposure_profile;
#[cfg(feature = "mc")]
pub use exposure::compute_stochastic_exposure_profile;

mod bridge;

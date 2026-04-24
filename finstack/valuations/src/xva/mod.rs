//! XVA compatibility layer over `finstack-margin`.
//!
//! Most of the implementation now lives in `finstack-margin`. This module keeps
//! the historical `finstack-valuations::xva::*` surface working, with a small
//! local bridge where valuations still accepts `Arc<dyn Instrument>`.

pub use finstack_margin::xva::{cva, netting, traits, types, Valuable};
pub use netting::{apply_collateral, apply_netting};
pub use types::{
    CsaTerms, ExposureDiagnostics, ExposureProfile, FundingConfig, XvaConfig, XvaNettingSet,
    XvaResult,
};
pub use types::{StochasticExposureConfig, StochasticExposureProfile};

pub mod exposure {
    //! Exposure-profile compatibility wrappers.

    use std::sync::Arc;

    use crate::instruments::DynInstrument;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_margin::xva::types::{ExposureProfile, XvaConfig, XvaNettingSet};

    use super::bridge::wrap_instruments;

    /// Preserve the historical valuations-side exposure API while the
    /// standalone margin crate transitions callers onto `Valuable`.
    pub fn compute_exposure_profile(
        instruments: &[Arc<DynInstrument>],
        market: &MarketContext,
        as_of: Date,
        config: &XvaConfig,
        netting_set: &XvaNettingSet,
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

    pub use finstack_margin::xva::exposure::compute_stochastic_exposure_profile;
}

pub use cva::{compute_bilateral_xva, compute_cva, compute_dva, compute_fva};
pub use exposure::compute_exposure_profile;
pub use exposure::compute_stochastic_exposure_profile;

mod bridge;

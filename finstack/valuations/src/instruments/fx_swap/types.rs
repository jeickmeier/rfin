//! FX Swap types and instrument integration.
//!
//! This file defines the `FxSwap` instrument shape and provides the
//! integration with the shared instrument trait via the `impl_instrument!`
//! macro. Core PV logic is delegated to `pricing::engine` to follow the
//! repository standards. Metrics live under `metrics/` and are registered
//! via the instrument metrics module.

use crate::instruments::common::parameters::FxUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use finstack_core::prelude::*;
use finstack_core::types::InstrumentId;
use finstack_core::F;

use super::parameters::FxSwapParams;

/// FX Swap instrument definition
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct FxSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign)
    pub base_currency: Currency,
    /// Quote currency (domestic)
    pub quote_currency: Currency,
    /// Near leg settlement date (spot leg)
    pub near_date: Date,
    /// Far leg settlement date (forward leg)
    pub far_date: Date,
    /// Notional amount in base currency (exchanged on near, reversed on far)
    pub base_notional: Money,
    /// Domestic discount curve id (quote currency)
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve id (base currency)
    pub foreign_disc_id: &'static str,
    /// Optional near leg FX rate (quote per base). If None, source from market.
    #[builder(optional)]
    pub near_rate: Option<F>,
    /// Optional far leg FX rate (quote per base). If None, source from forwards.
    #[builder(optional)]
    pub far_rate: Option<F>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl FxSwap {
    /// Create a new FX swap using parameter structs
    pub fn new(
        id: InstrumentId,
        swap_params: &FxSwapParams,
        underlying_params: &FxUnderlyingParams,
    ) -> Self {
        Self {
            id,
            base_currency: underlying_params.base_currency,
            quote_currency: underlying_params.quote_currency,
            near_date: swap_params.near_date,
            far_date: swap_params.far_date,
            base_notional: swap_params.base_notional,
            domestic_disc_id: underlying_params.domestic_disc_id,
            foreign_disc_id: underlying_params.foreign_disc_id,
            near_rate: swap_params.near_rate,
            far_rate: swap_params.far_rate,
            attributes: Attributes::new(),
        }
    }

    // Builder entrypoint is provided via derive
}

impl_instrument!(
    FxSwap,
    "FxSwap",
    pv = |s, curves, as_of| {
        // Delegate PV to the pricing engine to centralize pricing logic
        crate::instruments::fx_swap::pricing::engine::FxSwapPricer::pv(s, curves, as_of)
    }
);

impl crate::instruments::common::HasStringDiscountCurve for FxSwap {
    fn string_discount_curve_id(&self) -> &str {
        self.domestic_disc_id
    }
}

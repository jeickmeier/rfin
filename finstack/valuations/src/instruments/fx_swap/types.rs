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
use finstack_core::market_data::MarketContext;
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

    /// Compute present value in quote currency.
    ///
    /// Provides deterministic PV for `FxSwap` instruments following the library
    /// standards. The valuation computes the PV of both legs using discount curves
    /// and converts the foreign-leg PV to the domestic (quote) currency using the
    /// applicable spot rate or the instrument's provided near rate.
    ///
    /// Pricing formula:
    /// - Let base be the foreign currency and quote the domestic pricing currency
    /// - Near and far settlement dates: `near_date`, `far_date`
    /// - Base notional amount `N_base`
    /// - If `near_rate` is None, source spot from `FxMatrix`
    /// - If `far_rate` is None, compute forward via `F = S * DF_for(far) / DF_dom(far)`
    /// - Foreign leg PV (in base): `N_base * DF_for(near) - N_base * DF_for(far)`
    /// - Domestic leg PV (in quote): `-N_base * S * DF_dom(near) + N_base * F * DF_dom(far)`
    /// - Total PV in quote: `PV_for * S + PV_dom`
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        use finstack_core::money::fx::FxQuery;
        
        // Curves
        let domestic_disc = curves.get_discount(self.domestic_disc_id)?;
        let foreign_disc = curves.get_discount(self.foreign_disc_id)?;

        // Discount factors using curve's own day-count for stability
        let df_dom_near = domestic_disc.df_on_date_curve(self.near_date);
        let df_dom_far = domestic_disc.df_on_date_curve(self.far_date);
        let df_for_far = foreign_disc.df_on_date_curve(self.far_date);

        // Resolve near spot rate
        let spot = match self.near_rate {
            Some(rate) => rate,
            None => {
                let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })
                })?;
                (**fx_matrix)
                    .rate(FxQuery::new(self.base_currency, self.quote_currency, as_of))?
                    .rate
            }
        };

        // Resolve far forward rate
        let fwd = match self.far_rate {
            Some(rate) => rate,
            None => spot * df_for_far / df_dom_far,
        };

        // Currency safety
        if self.base_notional.currency() != self.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }
        let n_base = self.base_notional.amount();

        // Leg PVs (convert foreign leg cashflows at their own rates, discount domestically)
        let pv_foreign_dom = n_base * spot * df_dom_near - n_base * fwd * df_dom_far; // in quote
        let pv_dom_leg = -n_base * spot * df_dom_near + n_base * fwd * df_dom_far; // in quote

        // Sum domestic and converted foreign legs
        let total_pv = pv_foreign_dom + pv_dom_leg;
        Ok(Money::new(total_pv, self.quote_currency))
    }

    // Builder entrypoint is provided via derive
}

impl_instrument!(
    FxSwap,
    "FxSwap",
    pv = |s, curves, as_of| {
        // Call the instrument's own method
        s.npv(curves, as_of)
    }
);

impl crate::instruments::common::HasStringDiscountCurve for FxSwap {
    fn string_discount_curve_id(&self) -> &str {
        self.domestic_disc_id
    }
}


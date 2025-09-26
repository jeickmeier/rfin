//! FX Swap pricing engine.
//!
//! Provides deterministic PV for `FxSwap` instruments following the library
//! standards. The valuation computes the PV of both legs using discount curves
//! and converts the foreign-leg PV to the domestic (quote) currency using the
//! applicable spot rate or the instrument's provided near rate.
//!
//! Pricing formula:
//! - Let base be the foreign currency and quote the domestic pricing currency
//! - Near and far settlement dates: `near_date`, `far_date`
//! - Base notional amount `N_base`
//! - If `near_rate` is None, source spot from `FxMatrix`
//! - If `far_rate` is None, compute forward via `F = S * DF_for(far) / DF_dom(far)`
//! - Foreign leg PV (in base): `N_base * DF_for(near) - N_base * DF_for(far)`
//! - Domestic leg PV (in quote): `-N_base * S * DF_dom(near) + N_base * F * DF_dom(far)`
//! - Total PV in quote: `PV_for * S + PV_dom`

use crate::instruments::fx_swap::types::FxSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::{FxConversionPolicy, FxQuery};
use finstack_core::money::Money;
use finstack_core::Result;

/// Stateless pricing engine for `FxSwap`.
#[derive(Debug, Default, Clone, Copy)]
pub struct FxSwapPricer;

impl FxSwapPricer {
    /// Compute present value in quote currency.
    pub fn pv(inst: &FxSwap, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // Curves
        let domestic_disc =
            curves
                .get_discount(
                    inst.domestic_disc_id,
                )?;
        let foreign_disc =
            curves
                .get_discount(
                    inst.foreign_disc_id,
                )?;

        // Discount factors using curve's own day-count for stability
        let df_dom_near = domestic_disc.df_on_date_curve(inst.near_date);
        let df_dom_far = domestic_disc.df_on_date_curve(inst.far_date);
        let df_for_far = foreign_disc.df_on_date_curve(inst.far_date);

        // Resolve near spot rate
        let spot = match inst.near_rate {
            Some(rate) => rate,
            None => {
                let fx_matrix = curves.fx.as_ref().ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                        id: "fx_matrix".to_string(),
                    })
                })?;
                (**fx_matrix)
                    .rate(FxQuery {
                        from: inst.base_currency,
                        to: inst.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        // Resolve far forward rate
        let fwd = match inst.far_rate {
            Some(rate) => rate,
            None => spot * df_for_far / df_dom_far,
        };

        // Currency safety
        if inst.base_notional.currency() != inst.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }
        let n_base = inst.base_notional.amount();

        // Leg PVs (convert foreign leg cashflows at their own rates, discount domestically)
        let pv_foreign_dom = n_base * spot * df_dom_near - n_base * fwd * df_dom_far; // in quote
        let pv_dom_leg = -n_base * spot * df_dom_near + n_base * fwd * df_dom_far; // in quote

        // Sum domestic and converted foreign legs
        let total_pv = pv_foreign_dom + pv_dom_leg;
        Ok(Money::new(total_pv, inst.quote_currency))
    }
}

//! FX Swap types and implementations.

use crate::instruments::common::{FxSwapParams, FxUnderlyingParams};
use crate::instruments::traits::Attributes;
use finstack_core::money::fx::FxConversionPolicy;
use finstack_core::prelude::*;
use finstack_core::F;

/// FX Swap instrument definition
#[derive(Clone, Debug)]
pub struct FxSwap {
    /// Unique instrument identifier
    pub id: String,
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
    pub near_rate: Option<F>,
    /// Optional far leg FX rate (quote per base). If None, source from forwards.
    pub far_rate: Option<F>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl FxSwap {
    /// Create a new FX swap using parameter structs
    pub fn new(
        id: impl Into<String>,
        swap_params: &FxSwapParams,
        underlying_params: &FxUnderlyingParams,
    ) -> Self {
        Self {
            id: id.into(),
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

    /// Builder entrypoint
    pub fn builder() -> crate::instruments::fixed_income::fx_swap::mod_fx_swap::FxSwapBuilder {
        crate::instruments::fixed_income::fx_swap::mod_fx_swap::FxSwapBuilder::new()
    }
}

impl_instrument!(
    FxSwap,
    "FxSwap",
    pv = |s, curves, as_of| {
        // 1. Get discount curves
        let domestic_disc = curves.disc(s.domestic_disc_id)?;
        let foreign_disc = curves.disc(s.foreign_disc_id)?;

        // 2. Get year fractions
        let dc = finstack_core::dates::DayCount::Act365F;
        let t_near = dc
            .year_fraction(
                as_of,
                s.near_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_far = dc
            .year_fraction(
                as_of,
                s.far_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        // 3. Get discount factors
        let df_dom_near = domestic_disc.df(t_near);
        let df_dom_far = domestic_disc.df(t_far);
        let df_for_near = foreign_disc.df(t_near);
        let df_for_far = foreign_disc.df(t_far);

        // 4. Resolve near_rate (spot)
        let fx_matrix = curves.fx.as_ref().ok_or(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let near_rate = match s.near_rate {
            Some(rate) => rate,
            None => {
                (**fx_matrix)
                    .rate(finstack_core::money::fx::FxQuery {
                        from: s.base_currency,
                        to: s.quote_currency,
                        on: as_of,
                        policy: FxConversionPolicy::CashflowDate,
                        closure_check: None,
                        want_meta: false,
                    })?
                    .rate
            }
        };

        // 5. Resolve far_rate (forward)
        let far_rate = match s.far_rate {
            Some(rate) => rate,
            None => {
                // Forward rate F = S * df_foreign / df_domestic
                near_rate * df_for_far / df_dom_far
            }
        };

        // 6. Calculate PV of each leg
        if s.base_notional.currency() != s.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ));
        }
        let base_amt = s.base_notional.amount();

        // PV of foreign leg in foreign currency: (+CF at near_date, -CF at far_date)
        let pv_for_leg = base_amt * df_for_near - base_amt * df_for_far;

        // PV of domestic leg in domestic currency: (-CF at near_date, +CF at far_date)
        let pv_dom_leg = -base_amt * near_rate * df_dom_near + base_amt * far_rate * df_dom_far;

        // 7. Convert foreign leg PV to domestic currency and sum
        let spot_rate_val = (**fx_matrix)
            .rate(finstack_core::money::fx::FxQuery {
                from: s.base_currency,
                to: s.quote_currency,
                on: as_of,
                policy: FxConversionPolicy::CashflowDate,
                closure_check: None,
                want_meta: false,
            })?
            .rate;
        let spot_rate = spot_rate_val;

        let total_pv = pv_for_leg * spot_rate + pv_dom_leg;

        Ok(Money::new(total_pv, s.quote_currency))
    }
);

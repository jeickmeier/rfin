//! FX Swap types and instrument integration.
//!
//! This file defines the `FxSwap` instrument shape and provides the
//! integration with the shared instrument trait via the `impl_instrument!`
//! macro. Core PV logic is delegated to `pricing::engine` to follow the
//! repository standards. Metrics live under `metrics/` and are registered
//! via the instrument metrics module.

use crate::instruments::common::parameters::FxUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

use super::parameters::FxSwapParams;

/// FX Swap instrument definition
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
    pub domestic_discount_curve_id: CurveId,
    /// Foreign discount curve id (base currency)
    pub foreign_discount_curve_id: CurveId,
    /// Optional near leg FX rate (quote per base). If None, source from market.
    #[builder(optional)]
    pub near_rate: Option<f64>,
    /// Optional far leg FX rate (quote per base). If None, source from forwards.
    #[builder(optional)]
    pub far_rate: Option<f64>,
    /// Optional base currency calendar for spot/settlement adjustment metadata.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub base_calendar_id: Option<String>,
    /// Optional quote currency calendar for spot/settlement adjustment metadata.
    #[builder(default)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub quote_calendar_id: Option<String>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl FxSwap {
    /// Create a canonical example FX swap for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD swap with realistic forward points.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FXSWAP-EURUSD-6M"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .near_date(
                Date::from_calendar_date(2024, time::Month::January, 5)
                    .expect("Valid example date"),
            )
            .far_date(
                Date::from_calendar_date(2024, time::Month::July, 5).expect("Valid example date"),
            )
            .base_notional(Money::new(1_000_000.0, Currency::EUR))
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .near_rate_opt(Some(1.10))
            .far_rate_opt(Some(1.12))
            .attributes(Attributes::new())
            .build()
            .expect("Example FX swap construction should not fail")
    }

    /// Construct an FX swap from trade date and tenor using joint calendar spot roll.
    ///
    /// `spot_lag_days` defaults to 2 in most markets; supply calendar IDs to enforce
    /// base/quote business-day adjustment.
    #[allow(clippy::too_many_arguments)]
    pub fn from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        trade_date: Date,
        far_tenor_days: i64,
        base_notional: Money,
        domestic_discount_curve_id: impl Into<CurveId>,
        foreign_discount_curve_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        quote_calendar_id: Option<String>,
        spot_lag_days: u32,
        bdc: finstack_core::dates::BusinessDayConvention,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common::fx_dates::{adjust_joint_calendar, roll_spot_date};
        let near_date = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;
        let far_unadjusted = near_date + time::Duration::days(far_tenor_days);
        let far_date = adjust_joint_calendar(
            far_unadjusted,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;

        Self::builder()
            .id(id.into())
            .base_currency(base_currency)
            .quote_currency(quote_currency)
            .near_date(near_date)
            .far_date(far_date)
            .base_notional(base_notional)
            .domestic_discount_curve_id(domestic_discount_curve_id.into())
            .foreign_discount_curve_id(foreign_discount_curve_id.into())
            .base_calendar_id_opt(base_calendar_id)
            .quote_calendar_id_opt(quote_calendar_id)
            .attributes(Attributes::new())
            .build()
    }

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
            domestic_discount_curve_id: underlying_params.domestic_discount_curve_id.to_owned(),
            foreign_discount_curve_id: underlying_params.foreign_discount_curve_id.to_owned(),
            near_rate: swap_params.near_rate,
            far_rate: swap_params.far_rate,
            base_calendar_id: None,
            quote_calendar_id: None,
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
        let domestic_disc = curves.get_discount_ref(self.domestic_discount_curve_id.as_str())?;
        let foreign_disc = curves.get_discount_ref(self.foreign_discount_curve_id.as_str())?;

        // Discount factors from as_of for correct theta (curve-consistent date mapping).
        let df_dom_near = domestic_disc.df_between_dates(as_of, self.near_date)?;
        let df_dom_far = domestic_disc.df_between_dates(as_of, self.far_date)?;
        let df_for_near = foreign_disc.df_between_dates(as_of, self.near_date)?;
        let df_for_far = foreign_disc.df_between_dates(as_of, self.far_date)?;

        // Settlement checks
        let include_near = self.near_date >= as_of;
        let include_far = self.far_date >= as_of;

        // Resolve model spot from FX matrix if available; otherwise fall back to contract near rate
        let model_spot = if let Some(fx) = curves.fx() {
            (**fx)
                .rate(FxQuery::new(self.base_currency, self.quote_currency, as_of))?
                .rate
        } else if let Some(rate) = self.near_rate {
            rate
        } else {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::NotFound {
                    id: "fx_matrix".to_string(),
                },
            ));
        };

        // Contract rates default to model when not provided explicitly
        let contract_spot = self.near_rate.unwrap_or(model_spot);

        // Calculate model forward only if far leg is active or needed.
        // Covered interest parity: F = S × (DF_for_far/DF_for_near) / (DF_dom_far/DF_dom_near)
        // When r_dom > r_for, forward is at premium (F > S) as required by no-arbitrage.
        // Derivation: F = S × (1 + r_dom × T) / (1 + r_for × T) = S × DF_for / DF_dom
        let dom_ratio = if df_dom_near.abs() > 1e-12 {
            df_dom_far / df_dom_near
        } else {
            1.0
        };
        let for_ratio = if df_for_near.abs() > 1e-12 {
            df_for_far / df_for_near
        } else {
            1.0
        };
        let model_fwd = model_spot * for_ratio / dom_ratio;
        let contract_fwd = self.far_rate.unwrap_or(model_fwd);

        // Currency safety
        if self.base_notional.currency() != self.base_currency {
            return Err(finstack_core::Error::from(
                finstack_core::InputError::Invalid,
            ));
        }
        let n_base = self.base_notional.amount();

        // Leg PV decomposition in quote currency:
        // - Foreign leg: receive base currency at near, pay base currency at far
        // - Domestic leg: pay quote currency at near, receive quote currency at far
        // - Foreign leg discounted with foreign curve, then converted to quote currency
        // - Domestic leg discounted with domestic curve using contract rates

        // Foreign leg PV in base currency, then convert to quote currency
        // Only include flows that have not settled
        let mut pv_foreign_leg_base = 0.0;
        if include_near {
            pv_foreign_leg_base += n_base * df_for_near;
        }
        if include_far {
            pv_foreign_leg_base -= n_base * df_for_far;
        }
        let pv_foreign_dom = pv_foreign_leg_base * model_spot;

        // Domestic leg PV in quote currency using contract rates
        let mut pv_dom_leg = 0.0;
        if include_near {
            pv_dom_leg += -n_base * contract_spot * df_dom_near;
        }
        if include_far {
            pv_dom_leg += n_base * contract_fwd * df_dom_far;
        }

        // Sum domestic and converted foreign legs
        let total_pv = pv_foreign_dom + pv_dom_leg;
        Ok(Money::new(total_pv, self.quote_currency))
    }

    // Builder entrypoint is provided via derive
}

impl crate::instruments::common::traits::Instrument for FxSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxSwap
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for FxSwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.domestic_discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for FxSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

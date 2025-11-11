//! FX option instrument implementation using Garman–Kohlhagen model.

use crate::instruments::common::parameters::FxUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
// Pricing/greeks live in pricing engine; keep types minimal.
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

use super::calculator::{FxOptionCalculator, FxOptionGreeks};
use super::parameters::FxOptionParams;

/// FX option instrument (Garman-Kohlhagen model)
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxOption {
    pub id: InstrumentId,
    pub base_currency: Currency,
    pub quote_currency: Currency,
    pub strike: f64,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub expiry: Date,
    pub day_count: finstack_core::dates::DayCount,
    pub notional: Money,
    pub settlement: SettlementType,
    pub domestic_discount_curve_id: CurveId,
    pub foreign_discount_curve_id: CurveId,
    pub vol_surface_id: CurveId,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
// Uses domestic curve as the primary discount curve
impl crate::metrics::HasDiscountCurve for FxOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.domestic_discount_curve_id
    }
}

impl FxOption {
    /// Create a European call option on an FX pair with standard conventions.
    pub fn european_call(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: f64,
        expiry: Date,
        notional: Money,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            // Fallback for other pairs - use USD for both curves
            FxUnderlyingParams::new(base_currency, quote_currency, "USD-OIS", "USD-OIS")
        };
        Self::builder()
            .id(id.into())
            .base_currency(fx_underlying.base_currency)
            .quote_currency(fx_underlying.quote_currency)
            .strike(strike)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .notional(notional)
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(fx_underlying.domestic_discount_curve_id.to_owned())
            .foreign_discount_curve_id(fx_underlying.foreign_discount_curve_id.to_owned())
            .vol_surface_id(vol_surface_id.into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European call construction should not fail")
    }

    /// Create a European put option on an FX pair with standard conventions.
    pub fn european_put(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: f64,
        expiry: Date,
        notional: Money,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            // Fallback for other pairs - use USD for both curves
            FxUnderlyingParams::new(base_currency, quote_currency, "USD-OIS", "USD-OIS")
        };
        Self::builder()
            .id(id.into())
            .base_currency(fx_underlying.base_currency)
            .quote_currency(fx_underlying.quote_currency)
            .strike(strike)
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .notional(notional)
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(fx_underlying.domestic_discount_curve_id.to_owned())
            .foreign_discount_curve_id(fx_underlying.foreign_discount_curve_id.to_owned())
            .vol_surface_id(vol_surface_id.into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European put construction should not fail")
    }

    /// Create a new FX option using parameter structs
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &FxOptionParams,
        underlying_params: &FxUnderlyingParams,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            base_currency: underlying_params.base_currency,
            quote_currency: underlying_params.quote_currency,
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            day_count: finstack_core::dates::DayCount::Act365F,
            notional: option_params.notional,
            settlement: option_params.settlement,
            domestic_discount_curve_id: underlying_params.domestic_discount_curve_id.to_owned(),
            foreign_discount_curve_id: underlying_params.foreign_discount_curve_id.to_owned(),
            vol_surface_id: vol_surface_id.into(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a centralized calculator instance with default configuration.
    pub fn calculator(&self) -> FxOptionCalculator {
        FxOptionCalculator::default()
    }

    /// Compute present value using Garman–Kohlhagen model.
    pub fn npv(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.calculator().npv(self, market, as_of)
    }

    /// Compute present value (alias for npv, used by instrument trait).
    pub fn value(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.npv(market, as_of)
    }

    /// Compute greeks using Garman–Kohlhagen model.
    pub fn compute_greeks(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: Date,
    ) -> Result<FxOptionGreeks> {
        self.calculator().compute_greeks(self, curves, as_of)
    }

    /// Solve for implied volatility.
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        self.calculator()
            .implied_vol(self, curves, as_of, target_price, initial_guess)
    }
}

impl crate::instruments::common::traits::Instrument for FxOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxOption
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
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.value(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self, market, as_of, base_value, metrics,
        )
    }
}

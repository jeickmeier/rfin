//! FX option instrument implementation using Garman–Kohlhagen model.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
// Pricing/greeks live in pricing engine; keep types minimal.
use finstack_core::money::Money;
use finstack_core::F;

use super::parameters::FxOptionParams;

/// FX option underlying parameters used when constructing the instrument.
#[derive(Clone, Debug)]
pub struct FxUnderlyingParams {
    /// Base currency (being priced)
    pub base_currency: Currency,
    /// Quote currency (pricing currency)
    pub quote_currency: Currency,
    /// Domestic discount curve ID (quote currency)
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve ID (base currency)
    pub foreign_disc_id: &'static str,
}

impl FxUnderlyingParams {
    /// Create FX underlying parameters
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        domestic_disc_id: &'static str,
        foreign_disc_id: &'static str,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            domestic_disc_id,
            foreign_disc_id,
        }
    }

    /// Standard USD/EUR pair
    pub fn usd_eur() -> Self {
        Self::new(Currency::EUR, Currency::USD, "USD-OIS", "EUR-OIS")
    }

    /// Standard GBP/USD pair
    pub fn gbp_usd() -> Self {
        Self::new(Currency::GBP, Currency::USD, "USD-OIS", "GBP-OIS")
    }
}

/// FX option instrument (Garman-Kohlhagen model)
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct FxOption {
    pub id: String,
    pub base_currency: Currency,
    pub quote_currency: Currency,
    pub strike: F,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub expiry: Date,
    pub day_count: finstack_core::dates::DayCount,
    pub notional: Money,
    pub settlement: SettlementType,
    pub domestic_disc_id: &'static str,
    pub foreign_disc_id: &'static str,
    pub vol_id: &'static str,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl FxOption {
    /// Create a European call option on an FX pair with standard conventions.
    pub fn european_call(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: F,
        expiry: Date,
        notional: Money,
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
            .domestic_disc_id(fx_underlying.domestic_disc_id)
            .foreign_disc_id(fx_underlying.foreign_disc_id)
            .vol_id("FX-VOL")
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European call construction should not fail")
    }

    /// Create a European put option on an FX pair with standard conventions.
    pub fn european_put(
        id: impl Into<String>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: F,
        expiry: Date,
        notional: Money,
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
            .domestic_disc_id(fx_underlying.domestic_disc_id)
            .foreign_disc_id(fx_underlying.foreign_disc_id)
            .vol_id("FX-VOL")
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FX European put construction should not fail")
    }

    /// Create a new FX option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &FxOptionParams,
        underlying_params: &FxUnderlyingParams,
        vol_id: &'static str,
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
            domestic_disc_id: underlying_params.domestic_disc_id,
            foreign_disc_id: underlying_params.foreign_disc_id,
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    // Pricing and greeks moved to pricing::engine to keep type slim.
}

impl_instrument!(
    FxOption,
    "FxOption",
    pv = |s, curves, as_of| crate::instruments::fx_option::pricing::FxOptionPricer::npv(
        s, curves, as_of
    )
);

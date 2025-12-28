//! FX option instrument implementation using Garman–Kohlhagen model.

use crate::instruments::common::parameters::FxUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
// Pricing/greeks live in pricing engine; keep types minimal.
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Result;

use super::calculator::{FxOptionCalculator, FxOptionGreeks};
use super::parameters::FxOptionParams;

fn default_fx_underlying(base_currency: Currency, quote_currency: Currency) -> FxUnderlyingParams {
    // Fall back to currency-aware OIS curves instead of hardwiring USD legs.
    let domestic = CurveId::new(format!("{}-OIS", quote_currency));
    let foreign = CurveId::new(format!("{}-OIS", base_currency));
    FxUnderlyingParams::new(base_currency, quote_currency, domestic, foreign)
}

/// FX option instrument (Garman-Kohlhagen model)
#[derive(
    Clone,
    Debug,
    finstack_valuations_macros::FinancialBuilder,
    finstack_valuations_macros::Instrument,
)]
#[instrument(key = "FxOption", price_fn = "npv")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign currency)
    pub base_currency: Currency,
    /// Quote currency (domestic currency)
    pub quote_currency: Currency,
    /// Strike exchange rate
    pub strike: f64,
    /// Option type (call or put on base currency)
    pub option_type: OptionType,
    /// Exercise style (European or American)
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Notional amount in base currency
    pub notional: Money,
    /// Settlement type (physical or cash)
    pub settlement: SettlementType,
    /// Domestic currency discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign currency discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX volatility surface ID
    pub vol_surface_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
// Uses domestic curve as the primary discount curve
impl crate::metrics::HasDiscountCurve for FxOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.domestic_discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for FxOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .build()
    }
}

impl FxOption {
    /// Create a canonical example FX option for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD call option.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        Self::builder()
            .id(InstrumentId::new("FXOPT-EURUSD-CALL"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .strike(1.12)
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(
                Date::from_calendar_date(2024, time::Month::June, 21).expect("Valid example date"),
            )
            .day_count(DayCount::Act365F)
            .notional(Money::new(1_000_000.0, Currency::EUR))
            .settlement(SettlementType::Cash)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example FX option construction should not fail")
    }

    /// Create a European call option on an FX pair with standard conventions.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn european_call(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: f64,
        expiry: Date,
        notional: Money,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            default_fx_underlying(base_currency, quote_currency)
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
    }

    /// Create a European put option on an FX pair with standard conventions.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    pub fn european_put(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: f64,
        expiry: Date,
        notional: Money,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            default_fx_underlying(base_currency, quote_currency)
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
    }

    /// Create a European option from trade date using joint calendar spot roll and tenor.
    ///
    /// `spot_lag_days` defaults to T+2 in most markets. The expiry is rolled on the
    /// joint base/quote calendars using the provided business day convention.
    #[allow(clippy::too_many_arguments)]
    pub fn european_from_trade_date(
        id: impl Into<InstrumentId>,
        base_currency: Currency,
        quote_currency: Currency,
        strike: f64,
        trade_date: Date,
        expiry_tenor_days: i64,
        notional: Money,
        vol_surface_id: impl Into<CurveId>,
        base_calendar_id: Option<String>,
        quote_calendar_id: Option<String>,
        spot_lag_days: u32,
        bdc: finstack_core::dates::BusinessDayConvention,
        option_type: OptionType,
    ) -> finstack_core::Result<Self> {
        use crate::instruments::common::fx_dates::{adjust_joint_calendar, roll_spot_date};
        let spot_settle = roll_spot_date(
            trade_date,
            spot_lag_days,
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;
        let expiry = adjust_joint_calendar(
            spot_settle + time::Duration::days(expiry_tenor_days),
            bdc,
            base_calendar_id.as_deref(),
            quote_calendar_id.as_deref(),
        )?;

        let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
            FxUnderlyingParams::usd_eur()
        } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
            FxUnderlyingParams::gbp_usd()
        } else {
            super::types::default_fx_underlying(base_currency, quote_currency)
        };

        Self::builder()
            .id(id.into())
            .base_currency(fx_underlying.base_currency)
            .quote_currency(fx_underlying.quote_currency)
            .strike(strike)
            .option_type(option_type)
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.calculator().npv(self, market, as_of)
    }

    /// Compute present value (alias for npv, used by instrument trait).
    pub fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.npv(market, as_of)
    }

    /// Compute greeks using Garman–Kohlhagen model.
    pub fn compute_greeks(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> Result<FxOptionGreeks> {
        self.calculator().compute_greeks(self, curves, as_of)
    }

    /// Solve for implied volatility.
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> Result<f64> {
        self.calculator()
            .implied_vol(self, curves, as_of, target_price, initial_guess)
    }
}

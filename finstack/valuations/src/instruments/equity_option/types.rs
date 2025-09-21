//! Equity option instrument definition and Black–Scholes helpers.

// pricing formulas are implemented in the pricing engine; keep this module free of direct math imports
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use crate::instruments::underlying::EquityUnderlyingParams;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::Date;
//
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

use super::parameters::EquityOptionParams;

/// Equity option instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct EquityOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub strike: Money,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub expiry: Date,
    pub contract_size: F,
    pub day_count: finstack_core::dates::DayCount,
    pub settlement: SettlementType,
    pub disc_id: CurveId,
    pub spot_id: String,
    pub vol_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl EquityOption {
    /// Create a European call option with standard conventions.
    ///
    /// This convenience constructor eliminates the builder for the most common case.
    pub fn european_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        // Build directly using derive-generated builder setters
        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("European call construction should not fail")
    }

    /// Create a European put option with standard conventions.
    pub fn european_put(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("European put construction should not fail")
    }

    /// Create an American call option with standard conventions.
    pub fn american_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: F,
        expiry: Date,
        notional: Money,
        contract_size: F,
    ) -> Self {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::American)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .disc_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.dividend_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("American call construction should not fail")
    }

    /// Create a new equity option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &EquityOptionParams,
        underlying_params: &EquityUnderlyingParams,
        disc_id: CurveId,
        vol_id: CurveId,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            underlying_ticker: underlying_params.ticker.clone(),
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            contract_size: option_params.contract_size,
            day_count: finstack_core::dates::DayCount::Act365F,
            settlement: option_params.settlement,
            disc_id,
            spot_id: underlying_params.spot_id.clone(),
            vol_id,
            div_yield_id: underlying_params.dividend_yield_id.clone(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    pub fn black_scholes_price(
        &self,
        spot: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> finstack_core::Result<Money> {
        let unit_price = crate::instruments::equity_option::pricing::engine::price_bs_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        );
        Ok(Money::new(
            unit_price * self.contract_size,
            self.strike.currency(),
        ))
    }

    pub fn delta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        crate::instruments::equity_option::pricing::engine::greeks_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        )
        .delta
    }

    pub fn gamma(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        crate::instruments::equity_option::pricing::engine::greeks_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        )
        .gamma
    }

    pub fn vega(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        crate::instruments::equity_option::pricing::engine::greeks_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        )
        .vega
    }

    pub fn theta(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        crate::instruments::equity_option::pricing::engine::greeks_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        )
        .theta
    }

    pub fn rho(&self, spot: F, r: F, sigma: F, t: F, q: F) -> F {
        crate::instruments::equity_option::pricing::engine::greeks_unit(
            spot,
            self.strike.amount(),
            r,
            q,
            sigma,
            t,
            self.option_type,
        )
        .rho
    }
}

impl_instrument!(
    EquityOption,
    "EquityOption",
    pv = |s, curves, as_of| {
        crate::instruments::equity_option::pricing::engine::npv(s, curves, as_of)
    }
);

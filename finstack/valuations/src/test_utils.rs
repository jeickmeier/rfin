//! Shared helpers for unit tests to reduce boilerplate market setup.
#![allow(clippy::expect_used)]
use finstack_core::{
    currency::Currency,
    dates::Date,
    market_data::{
        surfaces::VolSurface,
        term_structures::{DiscountCurve, ForwardCurve, PriceCurve},
    },
    money::Money,
    types::{CurveId, InstrumentId},
};
use rust_decimal::Decimal;
use time::Month;

use crate::instruments::common::parameters::{EquityUnderlyingParams, FxUnderlyingParams};
use crate::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, CreditDefaultSwapBuilder, PayReceive, PremiumLegSpec,
    ProtectionLegSpec, RECOVERY_SENIOR_UNSECURED,
};
use crate::instruments::{Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType};
use crate::instruments::{EquityOption, FxOption};

/// Convenience date helper for tests.
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
        .expect("valid date")
}

/// Builder-based replacement for `EquityOption::european_call` (deprecated).
pub fn equity_option_european_call(
    id: impl Into<String>,
    ticker: impl Into<String>,
    strike: f64,
    expiry: Date,
    notional: Money,
    contract_size: f64,
) -> finstack_core::Result<EquityOption> {
    let ticker = ticker.into();
    let underlying = EquityUnderlyingParams::new(ticker.clone(), "EQUITY-SPOT", Currency::USD)
        .with_dividend_yield("EQUITY-DIVYIELD")
        .with_contract_size(contract_size);

    EquityOption::builder()
        .id(InstrumentId::new(id.into()))
        .underlying_ticker(underlying.ticker)
        .strike(Money::new(strike, notional.currency()))
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .contract_size(underlying.contract_size)
        .day_count(finstack_core::dates::DayCount::Act365F)
        .settlement(SettlementType::Cash)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .spot_id(underlying.spot_id)
        .vol_surface_id(CurveId::new("EQUITY-VOL"))
        .div_yield_id_opt(underlying.div_yield_id)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
}

/// Builder-based replacement for `EquityOption::european_put` (deprecated).
pub fn equity_option_european_put(
    id: impl Into<String>,
    ticker: impl Into<String>,
    strike: f64,
    expiry: Date,
    notional: Money,
    contract_size: f64,
) -> finstack_core::Result<EquityOption> {
    let ticker = ticker.into();
    let underlying = EquityUnderlyingParams::new(ticker.clone(), "EQUITY-SPOT", Currency::USD)
        .with_dividend_yield("EQUITY-DIVYIELD")
        .with_contract_size(contract_size);

    EquityOption::builder()
        .id(InstrumentId::new(id.into()))
        .underlying_ticker(underlying.ticker)
        .strike(Money::new(strike, notional.currency()))
        .option_type(OptionType::Put)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .contract_size(underlying.contract_size)
        .day_count(finstack_core::dates::DayCount::Act365F)
        .settlement(SettlementType::Cash)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .spot_id(underlying.spot_id)
        .vol_surface_id(CurveId::new("EQUITY-VOL"))
        .div_yield_id_opt(underlying.div_yield_id)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
}

/// Builder-based replacement for `EquityOption::american_call` (deprecated).
pub fn equity_option_american_call(
    id: impl Into<String>,
    ticker: impl Into<String>,
    strike: f64,
    expiry: Date,
    notional: Money,
    contract_size: f64,
) -> finstack_core::Result<EquityOption> {
    let ticker = ticker.into();
    let underlying = EquityUnderlyingParams::new(ticker.clone(), "EQUITY-SPOT", Currency::USD)
        .with_dividend_yield("EQUITY-DIVYIELD")
        .with_contract_size(contract_size);

    EquityOption::builder()
        .id(InstrumentId::new(id.into()))
        .underlying_ticker(underlying.ticker)
        .strike(Money::new(strike, notional.currency()))
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::American)
        .expiry(expiry)
        .contract_size(underlying.contract_size)
        .day_count(finstack_core::dates::DayCount::Act365F)
        .settlement(SettlementType::Cash)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .spot_id(underlying.spot_id)
        .vol_surface_id(CurveId::new("EQUITY-VOL"))
        .div_yield_id_opt(underlying.div_yield_id)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
}

/// Builder-based replacement for `FxOption::european_call` (deprecated).
pub fn fx_option_european_call(
    id: impl Into<InstrumentId>,
    base_currency: Currency,
    quote_currency: Currency,
    strike: f64,
    expiry: Date,
    notional: Money,
    vol_surface_id: impl Into<CurveId>,
) -> finstack_core::Result<FxOption> {
    let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
        FxUnderlyingParams::usd_eur()
    } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
        FxUnderlyingParams::gbp_usd()
    } else {
        let domestic = CurveId::new(format!("{}-OIS", quote_currency));
        let foreign = CurveId::new(format!("{}-OIS", base_currency));
        FxUnderlyingParams::new(base_currency, quote_currency, domestic, foreign)
    };

    FxOption::builder()
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
        .domestic_discount_curve_id(fx_underlying.domestic_discount_curve_id)
        .foreign_discount_curve_id(fx_underlying.foreign_discount_curve_id)
        .vol_surface_id(vol_surface_id.into())
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
}

/// Builder-based replacement for `FxOption::european_put` (deprecated).
pub fn fx_option_european_put(
    id: impl Into<InstrumentId>,
    base_currency: Currency,
    quote_currency: Currency,
    strike: f64,
    expiry: Date,
    notional: Money,
    vol_surface_id: impl Into<CurveId>,
) -> finstack_core::Result<FxOption> {
    let fx_underlying = if quote_currency == Currency::USD && base_currency == Currency::EUR {
        FxUnderlyingParams::usd_eur()
    } else if quote_currency == Currency::USD && base_currency == Currency::GBP {
        FxUnderlyingParams::gbp_usd()
    } else {
        let domestic = CurveId::new(format!("{}-OIS", quote_currency));
        let foreign = CurveId::new(format!("{}-OIS", base_currency));
        FxUnderlyingParams::new(base_currency, quote_currency, domestic, foreign)
    };

    FxOption::builder()
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
        .domestic_discount_curve_id(fx_underlying.domestic_discount_curve_id)
        .foreign_discount_curve_id(fx_underlying.foreign_discount_curve_id)
        .vol_surface_id(vol_surface_id.into())
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
}

/// Builder-based replacement for `CreditDefaultSwap::buy_protection` (deprecated).
#[allow(clippy::too_many_arguments)]
pub fn cds_buy_protection(
    id: impl Into<InstrumentId>,
    notional: Money,
    spread_bp: f64,
    start: Date,
    maturity: Date,
    discount_curve_id: impl Into<CurveId>,
    credit_id: impl Into<CurveId>,
) -> finstack_core::Result<CreditDefaultSwap> {
    let convention = CDSConvention::IsdaNa;
    let dc = convention.day_count();
    let freq = convention.frequency();
    let bdc = convention.business_day_convention();
    let stub = convention.stub_convention();

    let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
        finstack_core::Error::Validation(format!(
            "spread_bp {} cannot be represented as Decimal: {}",
            spread_bp, e
        ))
    })?;

    let cds = CreditDefaultSwapBuilder::new()
        .id(id.into())
        .notional(notional)
        .side(PayReceive::PayFixed)
        .convention(convention)
        .premium(PremiumLegSpec {
            start,
            end: maturity,
            freq,
            stub,
            bdc,
            calendar_id: Some(convention.default_calendar().to_string()),
            dc,
            spread_bp: spread_bp_decimal,
            discount_curve_id: discount_curve_id.into(),
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: credit_id.into(),
            recovery_rate: RECOVERY_SENIOR_UNSECURED,
            settlement_delay: convention.settlement_delay(),
        })
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()?;

    cds.validate()?;
    Ok(cds)
}

/// Builder-based replacement for `CreditDefaultSwap::sell_protection` (deprecated).
#[allow(clippy::too_many_arguments)]
pub fn cds_sell_protection(
    id: impl Into<InstrumentId>,
    notional: Money,
    spread_bp: f64,
    start: Date,
    maturity: Date,
    discount_curve_id: impl Into<CurveId>,
    credit_id: impl Into<CurveId>,
) -> finstack_core::Result<CreditDefaultSwap> {
    let convention = CDSConvention::IsdaNa;
    let dc = convention.day_count();
    let freq = convention.frequency();
    let bdc = convention.business_day_convention();
    let stub = convention.stub_convention();

    let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
        finstack_core::Error::Validation(format!(
            "spread_bp {} cannot be represented as Decimal: {}",
            spread_bp, e
        ))
    })?;

    let cds = CreditDefaultSwapBuilder::new()
        .id(id.into())
        .notional(notional)
        .side(PayReceive::ReceiveFixed)
        .convention(convention)
        .premium(PremiumLegSpec {
            start,
            end: maturity,
            freq,
            stub,
            bdc,
            calendar_id: Some(convention.default_calendar().to_string()),
            dc,
            spread_bp: spread_bp_decimal,
            discount_curve_id: discount_curve_id.into(),
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: credit_id.into(),
            recovery_rate: RECOVERY_SENIOR_UNSECURED,
            settlement_delay: convention.settlement_delay(),
        })
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()?;

    cds.validate()?;
    Ok(cds)
}

/// Build a flat discount curve with two knots: (0, 1.0) and (1y, exp(-rate)).
pub fn flat_discount(id: &str, as_of: Date, rate: f64) -> DiscountCurve {
    flat_discount_with_tenor(id, as_of, rate, 1.0)
}

/// Build a flat discount curve with a configurable far-tenor knot.
pub fn flat_discount_with_tenor(
    id: &str,
    as_of: Date,
    rate: f64,
    tenor_years: f64,
) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(as_of)
        .knots([(0.0, 1.0), (tenor_years, (-rate * tenor_years).exp())])
        .build()
        .expect("discount curve should build in tests")
}

/// Build a flat forward curve with two knots and a constant rate.
pub fn flat_forward_with_tenor(id: &str, as_of: Date, rate: f64, tenor_years: f64) -> ForwardCurve {
    ForwardCurve::builder(id, tenor_years)
        .base_date(as_of)
        .knots([(0.0, rate), (tenor_years, rate)])
        .build()
        .expect("forward curve should build in tests")
}

/// Build a flat price curve with a constant price level (for commodity forward prices).
pub fn flat_price_curve(id: &str, as_of: Date, price: f64, tenor_years: f64) -> PriceCurve {
    PriceCurve::builder(id)
        .base_date(as_of)
        .spot_price(price)
        .knots([(0.0, price), (tenor_years, price)])
        .build()
        .expect("price curve should build in tests")
}

/// Build a contango price curve (forward prices increase with time).
pub fn contango_price_curve(
    id: &str,
    as_of: Date,
    spot: f64,
    carry_rate: f64,
    tenor_years: f64,
) -> PriceCurve {
    // F(T) = S * exp(r * T)
    let far_price = spot * (carry_rate * tenor_years).exp();
    PriceCurve::builder(id)
        .base_date(as_of)
        .spot_price(spot)
        .knots([(0.0, spot), (tenor_years, far_price)])
        .build()
        .expect("price curve should build in tests")
}

/// Build a constant vol surface using provided expiries/strikes grid.
pub fn flat_vol_surface(id: &str, expiries: &[f64], strikes: &[f64], vol: f64) -> VolSurface {
    let mut builder = VolSurface::builder(id).expiries(expiries).strikes(strikes);
    for _ in expiries {
        builder = builder.row(&vec![vol; strikes.len()]);
    }
    builder.build().expect("vol surface should build in tests")
}

/// Calibration-specific helpers for integration tests.
pub mod calibration {
    use crate::calibration::api::schema::StepParams;
    use crate::calibration::step_runtime;
    use crate::calibration::{CalibrationConfig, CalibrationReport};
    use crate::market::quotes::market_quote::MarketQuote;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::Result;

    /// Execute a single calibration step for tests/benchmarks without engaging the full plan engine.
    ///
    /// This replaces the deprecated `calibration::execute_step_for_tests` shim.
    pub fn execute_step(
        params: &StepParams,
        quotes: &[MarketQuote],
        context: &MarketContext,
        global_config: &CalibrationConfig,
    ) -> Result<(MarketContext, CalibrationReport)> {
        step_runtime::execute_params_and_apply(params, quotes, context, global_config)
    }
}

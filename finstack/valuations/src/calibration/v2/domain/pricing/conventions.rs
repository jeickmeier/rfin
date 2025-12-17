use super::super::quotes::{InstrumentConventions, RatesQuote};
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
use finstack_core::Result;

use super::pricer::CalibrationPricer;

pub(crate) struct ResolvedCommon<'a> {
    pub settlement_days: i32,
    pub payment_delay_days: i32,
    pub reset_lag_days: i32,
    pub calendar_id: &'a str,
    pub bdc: BusinessDayConvention,
}

pub(crate) struct ResolvedMoneyMarket<'a> {
    pub common: ResolvedCommon<'a>,
    pub day_count: DayCount,
}

pub(crate) struct ResolvedSwapConventions<'a> {
    pub common: ResolvedCommon<'a>,
    pub fixed_freq: Tenor,
    pub float_freq: Tenor,
    pub fixed_dc: DayCount,
    pub float_dc: DayCount,
    pub index: &'a IndexId,
}

pub(crate) struct ResolvedBasisSwapConventions<'a> {
    pub currency: Currency,
    pub primary_freq: Tenor,
    pub reference_freq: Tenor,
    pub primary_dc: DayCount,
    pub reference_dc: DayCount,
    pub primary_index: &'a IndexId,
    pub reference_index: &'a IndexId,
}

pub(crate) struct ResolvedSettlement<'a> {
    pub settlement_days: i32,
    pub calendar_id: &'a str,
    pub bdc: BusinessDayConvention,
}

pub(crate) fn resolve_common<'a>(
    pricer: &'a CalibrationPricer,
    quote_conventions: &'a InstrumentConventions,
    currency: Currency,
) -> ResolvedCommon<'a> {
    let settlement_days = quote_conventions
        .settlement_days
        .or(pricer.settlement_days)
        .unwrap_or_else(|| CalibrationPricer::market_settlement_days(currency));

    let calendar_id = quote_conventions
        .calendar_id
        .as_deref()
        .or(pricer.calendar_id.as_deref())
        .unwrap_or_else(|| CalibrationPricer::market_calendar_id(currency));

    let bdc = quote_conventions
        .business_day_convention
        .or(pricer.business_day_convention)
        .unwrap_or_else(|| CalibrationPricer::market_business_day_convention(currency));

    ResolvedCommon {
        settlement_days,
        payment_delay_days: quote_conventions.effective_payment_delay_days(),
        reset_lag_days: quote_conventions.effective_reset_lag_days(),
        calendar_id,
        bdc,
    }
}

fn require_i32(field: Option<i32>, name: &'static str) -> Result<i32> {
    field.ok_or_else(|| finstack_core::Error::Validation(format!(
        "Instrument conventions require '{}' to be set",
        name
    )))
}

fn require_str<'a>(field: Option<&'a str>, name: &'static str) -> Result<&'a str> {
    field.ok_or_else(|| finstack_core::Error::Validation(format!(
        "Instrument conventions require '{}' to be set",
        name
    )))
}

fn require_bdc(field: Option<BusinessDayConvention>, name: &'static str) -> Result<BusinessDayConvention> {
    field.ok_or_else(|| finstack_core::Error::Validation(format!(
        "Instrument conventions require '{}' to be set",
        name
    )))
}

fn require_day_count(field: Option<DayCount>, name: &'static str) -> Result<DayCount> {
    field.ok_or_else(|| finstack_core::Error::Validation(format!(
        "Instrument conventions require '{}' to be set",
        name
    )))
}

fn require_tenor(field: Option<Tenor>, name: &'static str) -> Result<Tenor> {
    field.ok_or_else(|| finstack_core::Error::Validation(format!(
        "Instrument conventions require '{}' to be set",
        name
    )))
}

pub(crate) fn resolve_settlement_strict<'a>(
    quote_conventions: &'a InstrumentConventions,
    _currency: Currency,
) -> Result<ResolvedSettlement<'a>> {
    let settlement_days = require_i32(quote_conventions.settlement_days, "settlement_days")?;
    let calendar_id = require_str(quote_conventions.calendar_id.as_deref(), "calendar_id")?;
    let bdc = require_bdc(
        quote_conventions.business_day_convention,
        "business_day_convention",
    )?;
    Ok(ResolvedSettlement {
        settlement_days,
        calendar_id,
        bdc,
    })
}

pub(crate) fn resolve_common_strict<'a>(
    quote_conventions: &'a InstrumentConventions,
    _currency: Currency,
) -> Result<ResolvedCommon<'a>> {
    let settlement_days = require_i32(quote_conventions.settlement_days, "settlement_days")?;
    let payment_delay_days = require_i32(quote_conventions.payment_delay_days, "payment_delay_days")?;
    let reset_lag_days = require_i32(quote_conventions.reset_lag, "reset_lag")?;
    let calendar_id = require_str(quote_conventions.calendar_id.as_deref(), "calendar_id")?;
    let bdc = require_bdc(
        quote_conventions.business_day_convention,
        "business_day_convention",
    )?;

    Ok(ResolvedCommon {
        settlement_days,
        payment_delay_days,
        reset_lag_days,
        calendar_id,
        bdc,
    })
}

pub(crate) fn resolve_money_market<'a>(
    pricer: &'a CalibrationPricer,
    quote_conventions: &'a InstrumentConventions,
    currency: Currency,
) -> ResolvedMoneyMarket<'a> {
    let common = resolve_common(pricer, quote_conventions, currency);
    let day_count = quote_conventions
        .day_count
        .unwrap_or_else(|| InstrumentConventions::default_money_market_day_count(currency));

    ResolvedMoneyMarket { common, day_count }
}

pub(crate) fn resolve_swap_conventions<'a>(
    pricer: &'a CalibrationPricer,
    quote: &'a RatesQuote,
    currency: Currency,
) -> Result<ResolvedSwapConventions<'a>> {
    match quote {
        RatesQuote::Swap {
            conventions,
            fixed_leg_conventions,
            float_leg_conventions,
            ..
        } => {
            let common = resolve_common(pricer, conventions, currency);

            let fixed_freq = fixed_leg_conventions
                .payment_frequency
                .unwrap_or_else(|| InstrumentConventions::default_fixed_leg_frequency(currency));
            let float_freq = float_leg_conventions
                .payment_frequency
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_frequency(currency));

            let fixed_dc = fixed_leg_conventions
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_fixed_leg_day_count(currency));
            let float_dc = float_leg_conventions
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_day_count(currency));

            let index = float_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Swap quote requires float_leg_conventions.index to be set".to_string(),
                )
            })?;

            Ok(ResolvedSwapConventions {
                common,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index,
            })
        }
        _ => Err(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        )),
    }
}

pub(crate) fn resolve_basis_swap_conventions<'a>(
    _pricer: &'a CalibrationPricer,
    quote: &'a RatesQuote,
    currency: Currency,
) -> Result<ResolvedBasisSwapConventions<'a>> {
    match quote {
        RatesQuote::BasisSwap {
            conventions,
            primary_leg_conventions,
            reference_leg_conventions,
            ..
        } => {
            let basis_currency = conventions.currency.unwrap_or(currency);

            let primary_index = primary_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "BasisSwap quote requires primary_leg_conventions.index to be set".to_string(),
                )
            })?;
            let reference_index = reference_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "BasisSwap quote requires reference_leg_conventions.index to be set".to_string(),
                )
            })?;

            let primary_freq = primary_leg_conventions
                .payment_frequency
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_frequency(basis_currency));
            let reference_freq = reference_leg_conventions
                .payment_frequency
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_frequency(basis_currency));

            let primary_dc = primary_leg_conventions
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_day_count(basis_currency));
            let reference_dc = reference_leg_conventions
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_float_leg_day_count(basis_currency));

            Ok(ResolvedBasisSwapConventions {
                currency: basis_currency,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                primary_index,
                reference_index,
            })
        }
        _ => Err(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        )),
    }
}

pub(crate) fn resolve_basis_swap_conventions_strict<'a>(
    quote: &'a RatesQuote,
    currency: Currency,
) -> Result<ResolvedBasisSwapConventions<'a>> {
    match quote {
        RatesQuote::BasisSwap {
            conventions,
            primary_leg_conventions,
            reference_leg_conventions,
            ..
        } => {
            let basis_currency = conventions.currency.unwrap_or(currency);

            let primary_index = primary_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "BasisSwap quote requires primary_leg_conventions.index to be set".to_string(),
                )
            })?;
            let reference_index = reference_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "BasisSwap quote requires reference_leg_conventions.index to be set".to_string(),
                )
            })?;

            let primary_freq = require_tenor(
                primary_leg_conventions.payment_frequency,
                "primary_leg_conventions.payment_frequency",
            )?;
            let reference_freq = require_tenor(
                reference_leg_conventions.payment_frequency,
                "reference_leg_conventions.payment_frequency",
            )?;

            let primary_dc = require_day_count(
                primary_leg_conventions.day_count,
                "primary_leg_conventions.day_count",
            )?;
            let reference_dc = require_day_count(
                reference_leg_conventions.day_count,
                "reference_leg_conventions.day_count",
            )?;

            Ok(ResolvedBasisSwapConventions {
                currency: basis_currency,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                primary_index,
                reference_index,
            })
        }
        _ => Err(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        )),
    }
}



use super::super::quotes::{InstrumentConventions, RatesQuote};
use crate::calibration::domain::quotes::rate_index::{RateIndexConventions, RateIndexKind};
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
use finstack_core::Result;

use super::pricer::CalibrationPricer;

pub(crate) struct ResolvedCommon<'a> {
    pub settlement_days: i32,
    pub payment_delay_days: i32,
    pub reset_lag_days: i32,
    pub calendar_id: &'a str,
    pub fixing_calendar_id: &'a str,
    pub payment_calendar_id: &'a str,
    pub bdc: BusinessDayConvention,
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

    let fixing_calendar_id = quote_conventions
        .effective_fixing_calendar_id()
        .or(pricer.calendar_id.as_deref())
        .unwrap_or(calendar_id);

    let payment_calendar_id = quote_conventions
        .effective_payment_calendar_id()
        .or(pricer.calendar_id.as_deref())
        .unwrap_or(calendar_id);

    let bdc = quote_conventions
        .business_day_convention
        .or(pricer.business_day_convention)
        .unwrap_or_else(|| CalibrationPricer::market_business_day_convention(currency));

    ResolvedCommon {
        settlement_days,
        payment_delay_days: quote_conventions.effective_payment_delay_days(),
        reset_lag_days: quote_conventions.effective_reset_lag_days(),
        calendar_id,
        fixing_calendar_id,
        payment_calendar_id,
        bdc,
    }
}

fn resolve_common_for_swap<'a>(
    pricer: &'a CalibrationPricer,
    conventions: &'a InstrumentConventions,
    fixed_leg: &'a InstrumentConventions,
    float_leg: &'a InstrumentConventions,
    currency: Currency,
    float_index: &'a IndexId,
) -> ResolvedCommon<'a> {
    let index_conv = RateIndexConventions::for_index_with_currency(float_index, currency);

    let settlement_days = conventions
        .settlement_days
        .or(pricer.settlement_days)
        .unwrap_or_else(|| CalibrationPricer::market_settlement_days(currency));

    let calendar_id = conventions
        .calendar_id
        .as_deref()
        .or(fixed_leg.calendar_id.as_deref())
        .or(float_leg.calendar_id.as_deref())
        .or(pricer.calendar_id.as_deref())
        .unwrap_or_else(|| CalibrationPricer::market_calendar_id(currency));

    let bdc = conventions
        .business_day_convention
        .or(fixed_leg.business_day_convention)
        .or(float_leg.business_day_convention)
        .or(pricer.business_day_convention)
        .unwrap_or_else(|| CalibrationPricer::market_business_day_convention(currency));

    let fixing_calendar_id = conventions
        .fixing_calendar_id
        .as_deref()
        .or(float_leg.fixing_calendar_id.as_deref())
        .or(conventions.calendar_id.as_deref())
        .or(float_leg.calendar_id.as_deref())
        .or(pricer.calendar_id.as_deref())
        .unwrap_or(calendar_id);

    let payment_calendar_id = conventions
        .payment_calendar_id
        .as_deref()
        .or(fixed_leg.payment_calendar_id.as_deref())
        .or(float_leg.payment_calendar_id.as_deref())
        .or(conventions.calendar_id.as_deref())
        .or(fixed_leg.calendar_id.as_deref())
        .or(float_leg.calendar_id.as_deref())
        .or(pricer.calendar_id.as_deref())
        .unwrap_or(calendar_id);

    let payment_delay_days = conventions
        .payment_delay_days
        .or(fixed_leg.payment_delay_days)
        .or(float_leg.payment_delay_days)
        .unwrap_or_else(|| {
            if index_conv.kind == RateIndexKind::OvernightRfr {
                index_conv.default_payment_delay_days
            } else {
                0
            }
        });

    let reset_lag_days = float_leg
        .reset_lag
        .or(conventions.reset_lag)
        .unwrap_or_else(|| {
            if index_conv.kind == RateIndexKind::OvernightRfr {
                index_conv.default_reset_lag_days
            } else {
                -2
            }
        });

    ResolvedCommon {
        settlement_days,
        payment_delay_days,
        reset_lag_days,
        calendar_id,
        fixing_calendar_id,
        payment_calendar_id,
        bdc,
    }
}

fn require_i32(field: Option<i32>, name: &'static str) -> Result<i32> {
    field.ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "Instrument conventions require '{}' to be set",
            name
        ))
    })
}

fn require_str<'a>(field: Option<&'a str>, name: &'static str) -> Result<&'a str> {
    field.ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "Instrument conventions require '{}' to be set",
            name
        ))
    })
}

fn require_bdc(
    field: Option<BusinessDayConvention>,
    name: &'static str,
) -> Result<BusinessDayConvention> {
    field.ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "Instrument conventions require '{}' to be set",
            name
        ))
    })
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
            let fixed_freq = fixed_leg_conventions
                .payment_frequency
                .unwrap_or_else(|| InstrumentConventions::default_fixed_leg_frequency(currency));
            let index = float_leg_conventions.index.as_ref().ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Swap quote requires float_leg_conventions.index to be set".to_string(),
                )
            })?;

            let index_conv = RateIndexConventions::for_index_with_currency(index, currency);
            let float_freq = float_leg_conventions
                .payment_frequency
                .unwrap_or(index_conv.default_payment_frequency);

            let fixed_dc = fixed_leg_conventions
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_fixed_leg_day_count(currency));
            let float_dc = float_leg_conventions
                .day_count
                .unwrap_or(index_conv.day_count);

            let common = resolve_common_for_swap(
                pricer,
                conventions,
                fixed_leg_conventions,
                float_leg_conventions,
                currency,
                index,
            );

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
                    "BasisSwap quote requires reference_leg_conventions.index to be set"
                        .to_string(),
                )
            })?;

            let primary_index_conv =
                RateIndexConventions::for_index_with_currency(primary_index, basis_currency);
            let reference_index_conv =
                RateIndexConventions::for_index_with_currency(reference_index, basis_currency);

            let primary_freq = primary_leg_conventions
                .payment_frequency
                .unwrap_or(primary_index_conv.default_payment_frequency);
            let reference_freq = reference_leg_conventions
                .payment_frequency
                .unwrap_or(reference_index_conv.default_payment_frequency);

            let primary_dc = primary_leg_conventions
                .day_count
                .unwrap_or(primary_index_conv.day_count);
            let reference_dc = reference_leg_conventions
                .day_count
                .unwrap_or(reference_index_conv.day_count);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::domain::pricing::pricer::CalibrationPricer;
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn swap_defaults_use_index_conventions_for_ois() {
        let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("base_date");
        let pricer =
            CalibrationPricer::new(base_date, "USD-OIS").with_market_conventions(Currency::USD);

        let quote = RatesQuote::Swap {
            maturity: Date::from_calendar_date(2025, Month::January, 2).expect("maturity"),
            rate: 0.02,
            is_ois: false,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: InstrumentConventions::default(),
            float_leg_conventions: InstrumentConventions {
                index: Some(IndexId::new("USD-SOFR-OIS")),
                ..InstrumentConventions::default()
            },
        };

        let resolved = resolve_swap_conventions(&pricer, &quote, Currency::USD).expect("resolved");
        assert_eq!(resolved.float_freq, Tenor::annual());
        assert_eq!(resolved.common.payment_delay_days, 2);
        assert_eq!(resolved.common.reset_lag_days, 0);
    }

    #[test]
    fn basis_swap_defaults_use_index_tenors() {
        let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("base_date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");

        let quote = RatesQuote::BasisSwap {
            maturity: Date::from_calendar_date(2026, Month::January, 2).expect("maturity"),
            spread_bp: 10.0,
            conventions: InstrumentConventions::default(),
            primary_leg_conventions: InstrumentConventions {
                index: Some(IndexId::new("USD-SOFR-3M")),
                ..InstrumentConventions::default()
            },
            reference_leg_conventions: InstrumentConventions {
                index: Some(IndexId::new("USD-SOFR-6M")),
                ..InstrumentConventions::default()
            },
        };

        let resolved =
            resolve_basis_swap_conventions(&pricer, &quote, Currency::USD).expect("resolved");
        assert_eq!(resolved.primary_freq, Tenor::parse("3M").expect("3M"));
        assert_eq!(resolved.reference_freq, Tenor::parse("6M").expect("6M"));
    }
}

use crate::calibration::quotes::{InstrumentConventions, RatesQuote};
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub common: ResolvedCommon<'a>,
    pub currency: Currency,
    pub primary_freq: Tenor,
    pub reference_freq: Tenor,
    pub primary_dc: DayCount,
    pub reference_dc: DayCount,
    pub primary_index: &'a IndexId,
    pub reference_index: &'a IndexId,
}

pub(crate) fn default_calendar_for_currency(currency: Currency) -> &'static str {
    match currency {
        Currency::USD => "usny",
        Currency::EUR => "target2",
        Currency::GBP => "gblo",
        Currency::JPY => "jpto",
        Currency::CHF => "chzu",
        Currency::AUD => "ausy",
        Currency::CAD => "cato",
        Currency::NZD => "nzau",
        Currency::HKD => "hkex",
        Currency::SGD => "sgex",
        _ => "usny",
    }
}

pub(crate) fn default_settlement_days(currency: Currency) -> i32 {
    match currency {
        Currency::GBP => 0,
        Currency::AUD | Currency::CAD => 1,
        _ => 2,
    }
}

pub(crate) fn resolve_common<'a>(
    pricer: &CalibrationPricer,
    quote_conventions: &'a InstrumentConventions,
    currency: Currency,
) -> ResolvedCommon<'a> {
    let settlement_days = quote_conventions
        .settlement_days
        .or(pricer.settlement_days)
        .unwrap_or_else(|| default_settlement_days(currency));

    let calendar_id = quote_conventions
        .calendar_id
        .as_deref()
        .unwrap_or_else(|| default_calendar_for_currency(currency));

    ResolvedCommon {
        settlement_days,
        payment_delay_days: quote_conventions.payment_delay_days.unwrap_or(0),
        reset_lag_days: quote_conventions.reset_lag.unwrap_or(2),
        calendar_id,
        bdc: BusinessDayConvention::ModifiedFollowing,
    }
}

pub(crate) fn resolve_money_market<'a>(
    pricer: &CalibrationPricer,
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
    pricer: &CalibrationPricer,
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
    pricer: &CalibrationPricer,
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
            let common = resolve_common(pricer, conventions, basis_currency);

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
                common,
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
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn settlement_days_precedence_quote_over_pricer_over_default() {
        let base_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test date");

        // GBP default is 0, set pricer to 1, quote to 2
        let pricer = CalibrationPricer::new(base_date, "GBP-OIS")
            .with_settlement_days(1)
            .with_use_settlement_start(true);

        let quote_conv = InstrumentConventions::default().with_settlement_days(2);
        let resolved = resolve_common(&pricer, &quote_conv, Currency::GBP);
        assert_eq!(resolved.settlement_days, 2);

        let quote_conv_none = InstrumentConventions::default();
        let resolved2 = resolve_common(&pricer, &quote_conv_none, Currency::GBP);
        assert_eq!(resolved2.settlement_days, 1);

        let pricer_none = CalibrationPricer::new(base_date, "GBP-OIS");
        let resolved3 = resolve_common(&pricer_none, &quote_conv_none, Currency::GBP);
        assert_eq!(resolved3.settlement_days, 0);
    }

    #[test]
    fn calendar_precedence_quote_over_default() {
        let base_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");

        let quote_conv = InstrumentConventions::default().with_calendar_id("custom");
        let resolved = resolve_common(&pricer, &quote_conv, Currency::USD);
        assert_eq!(resolved.calendar_id, "custom");

        let quote_conv_none = InstrumentConventions::default();
        let resolved2 = resolve_common(&pricer, &quote_conv_none, Currency::USD);
        assert_eq!(resolved2.calendar_id, default_calendar_for_currency(Currency::USD));
    }

    #[test]
    fn common_defaults_are_stable() {
        let base_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test date");
        let pricer = CalibrationPricer::new(base_date, "USD-OIS");

        let c = InstrumentConventions::default();
        let resolved = resolve_common(&pricer, &c, Currency::USD);
        assert_eq!(resolved.payment_delay_days, 0);
        assert_eq!(resolved.reset_lag_days, 2);
        assert_eq!(resolved.bdc, BusinessDayConvention::ModifiedFollowing);
    }
}


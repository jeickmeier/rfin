#![allow(clippy::module_inception)]

use wasm_bindgen::prelude::*;

mod core;
mod valuations;

pub use core::cashflow::{
    JsAmortizationSpec as AmortizationSpec, JsCFKind as CFKind, JsCashFlow as CashFlow,
};
pub use core::config::{JsFinstackConfig as FinstackConfig, JsRoundingMode as RoundingMode};
pub use core::currency::JsCurrency as Currency;
pub use core::dates::add_months as addMonths;
pub use core::dates::available_calendar_codes as availableCalendarCodes;
pub use core::dates::available_calendars as availableCalendars;
pub use core::dates::build_fiscal_periods as buildFiscalPeriods;
pub use core::dates::build_periods as buildPeriods;
pub use core::dates::business_day_convention_from_name as businessDayConventionFromName;
pub use core::dates::business_day_convention_name as businessDayConventionName;
pub use core::dates::date_to_days_since_epoch as dateToDaysSinceEpoch;
pub use core::dates::days_in_month as daysInMonth;
pub use core::dates::days_since_epoch_to_date as daysSinceEpochToDate;
pub use core::dates::get_calendar as getCalendar;
pub use core::dates::is_leap_year as isLeapYear;
pub use core::dates::last_day_of_month as lastDayOfMonth;
pub use core::dates::next_cds_date as nextCdsDate;
pub use core::dates::next_equity_option_expiry as nextEquityOptionExpiry;
pub use core::dates::next_imm as nextImm;
pub use core::dates::next_imm_option_expiry as nextImmOptionExpiry;
pub use core::dates::{
    adjust, BusinessDayConvention, Calendar, Date, DayCount, DayCountContext, FiscalConfig,
    Frequency, Period, PeriodId, PeriodPlan, Schedule, ScheduleBuilder, StubKind,
};
pub use core::dates::{
    imm_option_expiry as immOptionExpiry, third_friday as thirdFriday,
    third_wednesday as thirdWednesday,
};
pub use core::market_data::{
    BaseCorrelationCurve, CreditIndexData, DiscountCurve, DividendEvent, DividendSchedule,
    DividendScheduleBuilder, ForwardCurve, FxConfig, FxConversionPolicy, FxMatrix, FxRateResult,
    HazardCurve, InflationCurve, MarketContext, MarketScalar, ScalarTimeSeries,
    SeriesInterpolation, VolSurface,
};
pub use core::math::{
    adaptiveQuadrature, adaptiveSimpson, binomialProbability, gaussLegendreIntegrate,
    gaussLegendreIntegrateAdaptive, gaussLegendreIntegrateComposite, logBinomialCoefficient,
    logFactorial, simpsonRule, trapezoidalRule, BrentSolver, GaussHermiteQuadrature, HybridSolver,
    NewtonSolver,
};
pub use core::money::JsMoney as Money;
pub use valuations::instruments::{Bond, Deposit};
pub use valuations::pricer::{
    create_standard_registry_js as createStandardRegistry, JsPricerRegistry as PricerRegistry,
};
pub use valuations::results::JsValuationResult as ValuationResult;

#[cfg(feature = "console_error_panic_hook")]
fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    init_panic_hook();
}

//! FX date utilities for joint calendar adjustments and spot rolls.

use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::calendar::types::Calendar;
use finstack_core::dates::{adjust, BusinessDayConvention, Date, HolidayCalendar};
use time::Duration;

fn weekends_only() -> Calendar {
    Calendar::new("weekends_only", "Weekends Only", true, &[])
}

fn resolve_calendar(cal_id: Option<&str>) -> CalendarWrapper {
    if let Some(id) = cal_id {
        if let Some(resolved) = CalendarRegistry::global().resolve_str(id) {
            return CalendarWrapper::Borrowed(resolved);
        }
    }
    CalendarWrapper::Owned(weekends_only())
}

enum CalendarWrapper {
    Borrowed(&'static dyn HolidayCalendar),
    Owned(Calendar),
}

impl CalendarWrapper {
    fn as_ref(&self) -> &dyn HolidayCalendar {
        match self {
            CalendarWrapper::Borrowed(c) => *c,
            CalendarWrapper::Owned(c) => c,
        }
    }
}

/// Adjust a date so it is a business day for both base and quote calendars.
///
/// Applies the business day convention sequentially on the base and quote
/// calendars until the date stabilizes or a small iteration limit is reached.
pub fn adjust_joint_calendar(
    date: Date,
    bdc: BusinessDayConvention,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> finstack_core::Result<Date> {
    let base_cal = resolve_calendar(base_cal_id);
    let quote_cal = resolve_calendar(quote_cal_id);

    let mut d = date;
    for _ in 0..5 {
        let adj_base = adjust(d, bdc, base_cal.as_ref())?;
        let adj_quote = adjust(adj_base, bdc, quote_cal.as_ref())?;
        if adj_quote == d {
            return Ok(adj_quote);
        }
        d = adj_quote;
    }
    Ok(d)
}

/// Roll a trade date to spot using joint calendar adjustment.
pub fn roll_spot_date(
    trade_date: Date,
    spot_lag_days: u32,
    bdc: BusinessDayConvention,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> finstack_core::Result<Date> {
    let prelim = trade_date + Duration::days(spot_lag_days as i64);
    adjust_joint_calendar(prelim, bdc, base_cal_id, quote_cal_id)
}

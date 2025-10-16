#[path = "dates/common.rs"]
mod common;
#[path = "dates/calendar_adjust.rs"]
mod calendar_adjust;
#[path = "dates/calendar_business_days_more.rs"]
mod calendar_business_days_more;
#[path = "dates/calendar_span.rs"]
mod calendar_span;
#[path = "dates/calendars.rs"]
mod calendars;
#[path = "dates/calendars_all.rs"]
mod calendars_all;
#[path = "dates/cny_edge_years.rs"]
mod cny_edge_years;
#[path = "dates/calendar_types.rs"]
mod calendar_types;
#[path = "dates/schedule_iter.rs"]
mod schedule_iter;
#[path = "dates/date_extensions.rs"]
mod date_extensions;
#[path = "dates/daycount_additional.rs"]
mod daycount_additional;

#[cfg(feature = "serde")]
#[path = "dates/calendar_serde.rs"]
mod calendar_serde;

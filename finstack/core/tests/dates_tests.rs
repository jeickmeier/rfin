#[path = "dates/calendar_adjust.rs"]
mod calendar_adjust;
#[path = "dates/calendar_business_days_more.rs"]
mod calendar_business_days_more;
#[path = "dates/calendar_composite.rs"]
mod calendar_composite;
#[path = "dates/calendar_generated.rs"]
mod calendar_generated;
#[path = "dates/calendar_registry.rs"]
mod calendar_registry;
#[path = "dates/calendar_rule.rs"]
mod calendar_rule;
#[path = "dates/calendar_span.rs"]
mod calendar_span;
#[path = "dates/calendar_types.rs"]
mod calendar_types;
#[path = "dates/calendars.rs"]
mod calendars;
#[path = "dates/calendars_all.rs"]
mod calendars_all;
#[path = "dates/cny_edge_years.rs"]
mod cny_edge_years;
#[path = "dates/common.rs"]
mod common;
#[path = "dates/date_extensions.rs"]
mod date_extensions;
#[path = "dates/daycount_additional.rs"]
mod daycount_additional;
#[path = "dates/schedule_iter.rs"]
mod schedule_iter;

#[cfg(feature = "serde")]
#[path = "dates/calendar_serde.rs"]
mod calendar_serde;

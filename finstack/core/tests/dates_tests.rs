//! Dates module integration tests.
//!
//! This test suite verifies market-standard correctness for:
//! - Calendar holiday calculations (USNY, TARGET2, NYSE, etc.)
//! - Business day conventions (Following, ModifiedFollowing, etc.)
//! - Day count conventions (30/360, Act/360, Act/365, Act/Act)
//! - Schedule generation with stub rules and EOM conventions
//! - Date extensions (fiscal year, quarters, weekdays)
//!
//! # Test Organization
//!
//! - `common`: Shared test helpers (TestCal, make_date, DAYCOUNT_TOLERANCE)
//! - `calendar_*`: Calendar functionality tests
//! - `calendars*`: Holiday calendar tests for specific markets
//! - `daycount_*`: Day count convention tests
//! - `schedule_iter`: Schedule generation tests
//! - `date_extensions`: DateExt trait tests

#[path = "dates/common.rs"]
mod common;

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
#[path = "dates/date_extensions.rs"]
mod date_extensions;
#[path = "dates/daycount_act365l.rs"]
mod daycount_act365l;
#[path = "dates/daycount_additional.rs"]
mod daycount_additional;
#[path = "dates/daycount_thirty360_eom.rs"]
mod daycount_thirty360_eom;
#[path = "dates/daycount_thirty_e360.rs"]
mod daycount_thirty_e360;
#[path = "dates/schedule_iter.rs"]
mod schedule_iter;

#[cfg(feature = "serde")]
#[path = "dates/calendar_serde.rs"]
mod calendar_serde;


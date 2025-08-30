//! Date-related Python bindings module
//!
//! This module contains all date-related functionality including:
//! - Core date handling
//! - Calendars and business day conventions
//! - Day count conventions
//! - Schedule generation

pub mod calendar;
pub mod date;
pub mod daycount;
pub mod periods;
pub mod schedule;

// Re-export commonly used items
pub use calendar::{py_available_calendars, PyBusDayConv, PyCalendar};
pub use date::{py_next_cds_date, py_next_imm, py_third_wednesday, PyDate};
pub use daycount::PyDayCount;
pub use periods::{
    py_build_fiscal_periods, py_build_periods, PyFiscalConfig, PyPeriod, PyPeriodId,
};
pub use schedule::{py_generate_schedule, PyFrequency, PyStubRule};

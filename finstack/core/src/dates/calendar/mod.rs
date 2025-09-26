//! Holiday calendar system with clean implementations.
//!
//! ## Overview
//!
//! This module provides a clean, unified holiday calendar system based on:
//! - JSON-defined calendar rules in `data/calendars/`
//! - A single `Calendar` struct for all calendars
//! - Rule-based holiday evaluation with optional bitset optimization
//! - Composite calendar support for combining multiple calendars
//!
//! ## Supported Date Range
//!
//! Holiday calendars are optimized for years **1970-2150** using generated bitsets.
//! Years outside this range fall back to runtime rule evaluation.
//!
//! ## Semantics
//!
//! - "Holiday" refers to non-working dates as defined by a specific market
//!   calendar. Many calendars also label weekends as holidays for convenience,
//!   while some intentionally ignore weekends in `is_holiday`.
//! - Independent of the above, [`HolidayCalendar::is_business_day`] always treats 
//!   Saturday/Sunday as non-business days and defers to `is_holiday` for market-specific closures.
//! - Prefer `is_business_day` for scheduling and adjustment logic.

pub(crate) mod algo;
pub mod business_days;
pub mod composite;
pub mod generated;
pub mod registry;
pub mod rule;
pub mod types;

// Re-export commonly used items for ergonomic imports
pub use types::Calendar;
pub use business_days::{adjust, available_calendars, BusinessDayConvention, HolidayCalendar};
pub use composite::{CompositeCalendar, CompositeMode};
pub use registry::CalendarRegistry;
pub use rule::{Direction, Observed, Rule};

// Include generated calendar implementations
include!(concat!(env!("OUT_DIR"), "/calendars.rs"));

// Re-export calendar constants with the original PascalCase names for backward compatibility
pub use {
    ASX as Asx,
    AUCE as Auce, 
    BRBD as Brbd,
    CATO as Cato,
    CHZH as Chzh,
    CME as Cme,
    CNBE as Cnbe,
    DEFR as Defr,
    GBLO as Gblo,
    HKEX as Hkex,
    HKHK as Hkhk,
    JPTO as Jpto,
    JPX as Jpx,
    NYSE as Nyse,
    SGSI as Sgsi,
    SIFMA as Sifma,
    SSE as Sse,
    TARGET2 as Target2,
    USNY as Usny,
};

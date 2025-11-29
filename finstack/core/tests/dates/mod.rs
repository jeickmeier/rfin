//! Dates module tests.
//!
//! This module contains tests for:
//! - Calendar rules (fixed, nth weekday, Easter, Chinese, Japanese equinox, spans)
//! - Calendar holiday calculations (USNY, TARGET2, NYSE, GBLO, HKHK, etc.)
//! - Business day conventions
//! - Day count conventions
//! - Schedule generation
//! - Date extensions

mod common;

// Calendar Rules
mod rules;
mod rules_coverage;
#[cfg(feature = "serde")]
mod rules_serde;

// Calendar Infrastructure
mod adjustment;
mod calendars;
mod composite;
mod generated;
mod registry;

// Day Count Conventions
mod daycount;

// Schedules & Utilities
mod extensions;
mod schedule;

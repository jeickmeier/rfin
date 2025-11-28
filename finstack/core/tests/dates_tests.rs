//! Dates module integration tests.
//!
//! This test suite verifies market-standard correctness for:
//! - Calendar rules (fixed, nth weekday, Easter, Chinese, Japanese equinox, spans)
//! - Calendar holiday calculations (USNY, TARGET2, NYSE, GBLO, HKHK, etc.)
//! - Business day conventions (Following, ModifiedFollowing, etc.)
//! - Day count conventions (30/360, Act/360, Act/365, Act/Act, Bus/252)
//! - Schedule generation with stub rules and EOM conventions
//! - Date extensions (fiscal year, quarters, weekdays)
//!
//! # Test Organization
//!
//! - `common`: Shared test helpers (TestCal, make_date, DAYCOUNT_TOLERANCE)
//! - `rules`: Calendar rule implementation tests
//! - `calendars`: Built-in regional calendar holiday tests
//! - `adjustment`: Business day convention tests
//! - `composite`: Composite calendar (union/intersection) tests
//! - `registry`: CalendarRegistry lookup and resolution tests
//! - `generated`: Low-level bitset and helper tests
//! - `daycount`: Day count convention tests
//! - `schedule`: Schedule generation tests
//! - `extensions`: DateExt trait tests

mod dates;

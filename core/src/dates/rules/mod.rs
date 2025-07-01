//! Generic holiday rule building blocks used to compose market calendars.
//!
//! The goal of this tiny DSL is to express common holiday patterns such as
//! "fixed date", "first Monday in February", or "Monday on/after 21 May"
//! without copy-pasting imperative algorithms into each calendar.
//!
//! All helpers are `#![no_std]` / allocation-free and usable in const
//! contexts where the `time` constructors are const (soon).

#![allow(clippy::many_single_char_names)]

use time::Date;

/// A single holiday rule that can decide whether a given [`Date`] is a holiday.
///
/// Calendars are just `&[&dyn HolidayRule]` slices evaluated with `any()`.
pub trait HolidayRule {
    /// Returns `true` if the rule declares the `date` a holiday.
    fn applies(&self, date: Date) -> bool;
}

mod fixed;
pub use fixed::{FixedDate, Observed};

mod nth_weekday;
pub use nth_weekday::NthWeekday;

mod weekday_shift;
pub use weekday_shift::WeekdayShift;

mod easter_offset;
pub use easter_offset::{
    AscensionThursday, EasterMonday, EasterOffset, GoodFriday, PentecostMonday,
};

mod fixed_range;
pub use fixed_range::FixedDateRange;

mod bridge;
pub use bridge::{BridgeDay, InLieuMonday};

mod span;
pub use span::HolidaySpan;

mod list_rule;
pub use list_rule::ListRule;

mod custom_func;
pub use custom_func::CustomFuncRule;

mod equinox_jp;
pub use equinox_jp::{VernalEquinoxJP, AutumnalEquinoxJP};

mod chinese_lunar;
pub use chinese_lunar::{ChineseNewYear, QingMing, BuddhasBirthday};

// TODO: upcoming rules
// pub mod nth_weekday;
// pub mod weekday_shift;
// pub mod easter;

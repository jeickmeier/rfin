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
pub use equinox_jp::{AutumnalEquinoxJP, VernalEquinoxJP};

mod chinese_lunar;
pub use chinese_lunar::{BuddhasBirthday, ChineseNewYear, QingMing};

// TODO: upcoming rules
// pub mod nth_weekday;
// pub mod weekday_shift;
// pub mod easter;

// -----------------------------------------------------------------------------
// Unified enum wrapper – exposes a single `DateRule` for end-users while the
// individual structs remain available internally.  Each variant wraps one of
// the existing rule types and simply delegates [`HolidayRule::applies`].
// -----------------------------------------------------------------------------

/// Convenience wrapper consolidating the most common holiday rule types behind
/// a single enum.  This allows users to store heterogeneous rules in
/// `&[DateRule]` slices without the indirection of `Box<dyn HolidayRule>`.
#[derive(Debug, Clone, Copy)]
pub enum DateRule {
    /// Fixed calendar date (e.g. **1-Jan**).  See [`FixedDate`].
    FixedDate(FixedDate),
    /// Nth weekday in a month (e.g. **last Monday in August**).  See [`NthWeekday`].
    NthWeekday(NthWeekday),
    /// Shifts holidays falling on weekend to neighbouring weekdays. See [`WeekdayShift`].
    WeekdayShift(WeekdayShift),
    /// Day offset relative to Easter Monday. See [`EasterOffset`].
    EasterOffset(EasterOffset),
    /// User-supplied function rule. See [`CustomFuncRule`].
    CustomFunc(CustomFuncRule),
}

impl HolidayRule for DateRule {
    fn applies(&self, date: Date) -> bool {
        match self {
            DateRule::FixedDate(r) => r.applies(date),
            DateRule::NthWeekday(r) => r.applies(date),
            DateRule::WeekdayShift(r) => r.applies(date),
            DateRule::EasterOffset(r) => r.applies(date),
            DateRule::CustomFunc(r) => r.applies(date),
        }
    }
}

// ***** From conversions for ergonomic `.into()` on rule creation *****

impl From<FixedDate> for DateRule {
    fn from(r: FixedDate) -> Self {
        DateRule::FixedDate(r)
    }
}

impl From<NthWeekday> for DateRule {
    fn from(r: NthWeekday) -> Self {
        DateRule::NthWeekday(r)
    }
}

impl From<WeekdayShift> for DateRule {
    fn from(r: WeekdayShift) -> Self {
        DateRule::WeekdayShift(r)
    }
}

impl From<EasterOffset> for DateRule {
    fn from(r: EasterOffset) -> Self {
        DateRule::EasterOffset(r)
    }
}

impl From<CustomFuncRule> for DateRule {
    fn from(r: CustomFuncRule) -> Self {
        DateRule::CustomFunc(r)
    }
}

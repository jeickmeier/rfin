//! Internal schedule date generation machinery.
//!
//! Contains [`BuilderInternal`] and helper functions for producing raw date
//! sequences from a frequency / stub / EOM specification.  This is consumed
//! exclusively by the public [`ScheduleBuilder`](super::ScheduleBuilder).

use smallvec::SmallVec;
use time::{Date, Duration};

use super::next_imm;
use crate::dates::date_extensions::DateExt;
use crate::dates::schedule_iter::StubKind;
use crate::dates::Tenor;

/// Small helper alias when we need to pre-buffer (used only for `ShortFront`).
type Buffer = SmallVec<[Date; 32]>;

/// Apply End-of-Month (EOM) convention to a date.
fn apply_eom(date: Date) -> Date {
    date.end_of_month()
}

#[inline]
fn maybe_eom(eom: bool, d: Date) -> Date {
    if eom {
        apply_eom(d)
    } else {
        d
    }
}

#[inline]
fn push_if_new(buf: &mut Buffer, d: Date) {
    if buf.last().copied() != Some(d) {
        buf.push(d)
    }
}

/// Check if a date is a CDS roll date (20th of Mar/Jun/Sep/Dec).
pub(super) fn is_cds_roll_date(date: Date) -> bool {
    crate::dates::imm::is_cds_date(date)
}

/// Check if a date is a standard IMM date (third Wednesday of Mar/Jun/Sep/Dec).
pub(super) fn is_imm_roll_date(date: Date) -> bool {
    crate::dates::imm::is_imm_date(date)
}

/// Generate IMM dates (third Wednesday of Mar/Jun/Sep/Dec) within the given range.
///
/// Unlike regular schedule generation which adds fixed intervals, this function
/// computes the actual third Wednesday of each quarterly month to handle the
/// variable day-of-month correctly.
pub(super) fn generate_imm_dates(start: Date, end: Date) -> Vec<Date> {
    let mut dates = Vec::new();

    let first_imm = if is_imm_roll_date(start) {
        start
    } else {
        next_imm(start)
    };

    if first_imm > end {
        return dates;
    }

    dates.push(first_imm);

    let mut current = first_imm;
    loop {
        let next = next_imm(current);
        if next > end {
            break;
        }
        dates.push(next);
        current = next;
    }

    dates
}

/// Enforce strictly increasing, duplicate-free dates while preserving original order.
/// Drops any consecutive duplicates and any dates that would not increase.
pub(super) fn enforce_monotonic_and_dedup(dates: &mut Vec<Date>) {
    if dates.is_empty() {
        return;
    }
    let mut write = 0;
    for read in 1..dates.len() {
        if dates[read] > dates[write] {
            write += 1;
            if read != write {
                dates[write] = dates[read];
            }
        }
    }
    dates.truncate(write + 1);
}

// ---------------------------------------------------------------------------
// BuilderInternal – raw date sequence generator
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(super) struct BuilderInternal {
    pub start: Date,
    pub end: Date,
    pub freq: Tenor,
    pub stub: StubKind,
    pub eom: bool,
}

impl BuilderInternal {
    pub(super) fn generate(self) -> crate::Result<Vec<Date>> {
        if self.start >= self.end {
            return Err(crate::error::InputError::InvalidScheduleRange {
                start: self.start,
                end: self.end,
            }
            .into());
        }
        match self.stub {
            StubKind::ShortFront => self.gen_short_front(),
            StubKind::LongFront => self.gen_long_front(),
            StubKind::LongBack => self.gen_long_back(),
            StubKind::None => self.gen_regular(),
            StubKind::ShortBack => self.gen_short_back(),
        }
    }

    fn add_tenor(self, date: Date, count: i32) -> crate::Result<Date> {
        let tenor = self.freq;
        if count == 1 {
            tenor.add_to_date(date, None, super::BusinessDayConvention::Unadjusted)
        } else if count == -1 {
            Ok(match tenor.unit {
                crate::dates::TenorUnit::Months => date.add_months(-(tenor.count as i32)),
                crate::dates::TenorUnit::Years => date.add_months(-(tenor.count as i32) * 12),
                crate::dates::TenorUnit::Weeks => date - Duration::weeks(tenor.count as i64),
                crate::dates::TenorUnit::Days => date - Duration::days(tenor.count as i64),
            })
        } else {
            Ok(date)
        }
    }

    fn gen_regular(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let (mut dt, end) = (
            maybe_eom(self.eom, self.start),
            maybe_eom(self.eom, self.end),
        );
        buf.push(dt);
        while dt < end {
            let next = self.add_tenor(dt, 1)?;
            if next > end {
                return Err(crate::error::InputError::NonIntegerScheduleTenor.into());
            }
            dt = maybe_eom(self.eom, next);
            push_if_new(&mut buf, dt);
        }
        Ok(buf.into_vec())
    }

    fn gen_short_back(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let (mut dt, end) = (
            maybe_eom(self.eom, self.start),
            maybe_eom(self.eom, self.end),
        );
        buf.push(dt);
        while dt < end {
            let next = self.add_tenor(dt, 1)?;
            dt = maybe_eom(self.eom, next);
            if dt > end {
                push_if_new(&mut buf, end);
                break;
            }
            push_if_new(&mut buf, dt);
        }
        Ok(buf.into_vec())
    }

    fn gen_short_front(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.end;
        let target = self.start;
        loop {
            let date_to_add = maybe_eom(self.eom, dt);
            push_if_new(&mut buf, date_to_add);
            if dt == target {
                break;
            }
            let prev = self.add_tenor(dt, -1)?;
            dt = if prev < target { target } else { prev };
        }
        buf.as_mut_slice().reverse();
        Ok(buf.into_vec())
    }

    fn gen_long_front(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let mut anchors = Vec::new();
        let mut dt = self.end;
        anchors.push(dt);
        while dt > self.start {
            let prev = self.add_tenor(dt, -1)?;
            if prev >= self.start {
                dt = prev;
                anchors.push(dt);
            } else {
                break;
            }
        }
        buf.push(maybe_eom(self.eom, self.start));
        for &a in anchors.iter().rev() {
            let d = maybe_eom(self.eom, a);
            push_if_new(&mut buf, d);
        }
        Ok(buf.into_vec())
    }

    fn gen_long_back(self) -> crate::Result<Vec<Date>> {
        let mut buf: Buffer = Buffer::new();
        let mut dt = self.start;
        buf.push(maybe_eom(self.eom, dt));
        while dt < self.end {
            let next = self.add_tenor(dt, 1)?;
            let next_after = self.add_tenor(next, 1)?;
            if next_after > self.end {
                let end_date = maybe_eom(self.eom, self.end);
                push_if_new(&mut buf, end_date);
                break;
            } else {
                let d = maybe_eom(self.eom, next);
                push_if_new(&mut buf, d);
                dt = next;
            }
        }
        Ok(buf.into_vec())
    }
}

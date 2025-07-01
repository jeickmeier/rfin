use crate::dates::calendar::HolidayCalendar;
use crate::dates::rules::*;
use time::{Date, Month};

/// China inter-bank settlement calendar (code: CNBE).
///
/// Implements the major State-Council holiday blocks for modern years.
/// This *approximation* covers:
/// • New-Year block (1–3 Jan)
/// • Spring-Festival (Chinese New Year) – first 7 days
/// • Qing Ming (≈ 4 Apr) – single day
/// • Labour-Day block (1–5 May)
/// • National-Day block (1–7 Oct)
///
/// Dragon-Boat & Mid-Autumn and weekend shift-swap working days are
/// TODO for a future release.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cnbe;

impl HolidayCalendar for Cnbe {
    fn is_holiday(&self, date: Date) -> bool {
        // New Year – 3-day block starting 1-Jan
        if HolidaySpan::new(FixedDate::new(Month::January, 1), 3).applies(date) {
            return true;
        }

        // Spring Festival – 7-day block from Lunar New Year
        if HolidaySpan::new(ChineseNewYear, 7).applies(date) {
            return true;
        }

        // Qing Ming (Tomb-Sweeping Day) – single day (~4 Apr)
        if QingMing.applies(date) {
            return true;
        }

        // Labour Day – 5-day block from 1-May
        if HolidaySpan::new(FixedDate::new(Month::May, 1), 5).applies(date) {
            return true;
        }

        // National Day – 7-day block from 1-Oct
        if HolidaySpan::new(FixedDate::new(Month::October, 1), 7).applies(date) {
            return true;
        }

        // TODO: Dragon-Boat & Mid-Autumn 3-day blocks; weekend shift-swap rules.

        false
    }
} 
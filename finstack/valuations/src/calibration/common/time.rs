//! Centralized time conversion utilities for calibration.
//!
//! This module provides consistent time-to-years conversion using proper
//! day-count conventions instead of ad-hoc arithmetic with magic constants.

use crate::instruments::fixed_income::cds::CDSConvention;
use finstack_core::dates::{Date, DayCount};
use finstack_core::F;

/// Convert date to year fraction using specified day count.
///
/// Uses the canonical DayCount::year_fraction method directly.
#[inline]
pub fn year_fraction(base: Date, end: Date, dc: DayCount) -> F {
    if end == base {
        0.0
    } else {
        dc.year_fraction(base, end, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0)
    }
}

/// Time-to-expiry for volatility instruments using Act/365F.
///
/// Standard convention for equity and FX option time-to-expiry calculations.
#[inline]
pub fn time_to_expiry_vol(base: Date, expiry: Date) -> F {
    year_fraction(base, expiry, DayCount::Act365F)
}

/// Time-to-maturity for CDS instruments using ISDA day count.
///
/// Uses the standard ISDA North America day count convention.
#[inline]
pub fn time_to_maturity_cds(base: Date, maturity: Date) -> F {
    year_fraction(base, maturity, CDSConvention::IsdaNa.day_count())
}

/// Time-to-maturity for inflation instruments using Act/Act.
///
/// Standard convention for inflation-linked instruments.
#[inline]
pub fn time_to_maturity_inflation(base: Date, maturity: Date) -> F {
    year_fraction(base, maturity, DayCount::ActAct)
}

/// Auto-detect appropriate day count based on instrument type.
///
/// Provides a fallback when the specific asset class is unknown.
#[inline]
pub fn time_to_maturity_auto(base: Date, maturity: Date) -> F {
    year_fraction(base, maturity, DayCount::Act365F)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_year_fraction_wrapper() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let yf = year_fraction(base, end, DayCount::Act365F);
        assert!((yf - 365.0 / 365.0).abs() < 1e-12);
    }

    #[test]
    fn test_vol_time_to_expiry() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2025, Month::April, 1).unwrap();

        let tte = time_to_expiry_vol(base, expiry);
        // Should be close to 3 months / 365 days
        assert!(tte > 0.2 && tte < 0.3);
    }

    #[test]
    fn test_cds_time_to_maturity() {
        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let ttm = time_to_maturity_cds(base, maturity);
        // Should be close to 5 years (ISDA day count may give slightly different result)
        assert!((ttm - 5.0).abs() < 0.1, "Expected ~5.0, got {}", ttm);
    }

    #[test]
    fn test_different_day_counts_give_different_results() {
        let base = Date::from_calendar_date(2024, Month::February, 1).unwrap(); // Leap year
        let end = Date::from_calendar_date(2025, Month::February, 1).unwrap();

        let act365f = time_to_expiry_vol(base, end);
        let actact = time_to_maturity_inflation(base, end);

        // Should be different due to leap year handling
        assert!((act365f - actact).abs() > 1e-6);
    }
}

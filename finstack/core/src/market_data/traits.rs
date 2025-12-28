//! Minimal traits for market data polymorphism.
//!
//! Defines lightweight trait interfaces for discount curves, forward curves,
//! and survival curves. These traits enable polymorphic pricing code while
//! keeping most functionality as concrete methods for better discoverability.
//!
//! # Design Philosophy
//!
//! - **Minimal trait surface**: Only methods needed for polymorphism
//! - **Concrete types preferred**: Most methods live on concrete curve types
//! - **Zero-cost abstraction**: Trait objects use dynamic dispatch only when needed

use crate::dates::{Date, DayCount, DayCountCtx};

// -----------------------------------------------------------------------------
// Minimal traits for polymorphism only
// -----------------------------------------------------------------------------

/// Minimal trait for discount curve polymorphism.
/// Only implement this where you need to accept different discount curve types.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::traits::{Discounting, TermStructure};
/// use finstack_core::types::CurveId;
/// use finstack_core::dates::Date;
/// use time::macros::date;
///
/// struct FlatCurve {
///     id: CurveId,
///     df_const: f64,
/// }
///
/// impl FlatCurve {
///     fn new(id: &str, df_const: f64) -> Self {
///         Self { id: CurveId::from(id), df_const }
///     }
/// }
///
/// impl TermStructure for FlatCurve {
///     fn id(&self) -> &CurveId { &self.id }
/// }
///
/// impl Discounting for FlatCurve {
///     fn base_date(&self) -> Date {
///         date!(2025 - 01 - 01)
///     }
///     fn df(&self, _t: f64) -> f64 { self.df_const }
/// }
///
/// let curve = FlatCurve::new("USD", 0.97);
/// assert!(curve.df(1.0) < 1.0);
/// ```
pub trait Discounting: TermStructure {
    /// Base (valuation) date of the curve.
    fn base_date(&self) -> Date;
    /// Discount factor at time `t` (year fraction from the base date).
    fn df(&self, t: f64) -> f64;

    /// Day count convention used by the curve for time-to-maturity calculations.
    ///
    /// This is the day count that should be used when converting dates to year
    /// fractions for looking up discount factors. Defaults to `Act365F` which
    /// is the most common convention for discount curves.
    ///
    /// **Important**: For consistent pricing, code that discounts cashflows should
    /// use the curve's day count (via this method) rather than the instrument's
    /// accrual day count.
    fn day_count(&self) -> DayCount {
        DayCount::Act365F
    }

    /// Discount factor from `from` to `to` using the curve's day-count.
    ///
    /// Canonical helper for the common "relative DF" pattern:
    /// `DF(from→to) = DF(0→to) / DF(0→from)`.
    ///
    /// Returns `Ok(1.0)` when `from == to`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Day-count year fraction calculation fails
    /// - Either discount factor is non-finite or non-positive
    #[inline]
    fn df_between_dates(&self, from: Date, to: Date) -> crate::Result<f64> {
        if from == to {
            return Ok(1.0);
        }

        let dc = self.day_count();
        let base = self.base_date();

        let t_from = dc.year_fraction(base, from, DayCountCtx::default())?;
        let df_from = self.df(t_from);
        if !df_from.is_finite() || df_from <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'from' date ({from}): {df_from}"
            )));
        }

        let t_to = dc.year_fraction(base, to, DayCountCtx::default())?;
        let df_to = self.df(t_to);
        if !df_to.is_finite() || df_to <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'to' date ({to}): {df_to}"
            )));
        }

        Ok(df_to / df_from)
    }

    /// Discount factor from `from_t` to `to_t` where `t` is year-fraction
    /// from the curve base date.
    ///
    /// Canonical helper for `DF(from_t→to_t) = DF(0→to_t) / DF(0→from_t)`.
    ///
    /// Returns `Ok(1.0)` when `from_t == to_t`.
    ///
    /// # Errors
    ///
    /// Returns an error if either discount factor is non-finite or non-positive.
    #[inline]
    fn df_between_times(&self, from_t: f64, to_t: f64) -> crate::Result<f64> {
        if from_t == to_t {
            return Ok(1.0);
        }

        let df_from = self.df(from_t);
        if !df_from.is_finite() || df_from <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor at 'from_t' ({from_t}): {df_from}"
            )));
        }

        let df_to = self.df(to_t);
        if !df_to.is_finite() || df_to <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor at 'to_t' ({to_t}): {df_to}"
            )));
        }

        Ok(df_to / df_from)
    }
}

/// Minimal trait for forward curve polymorphism where needed.
/// Most code should call methods directly on `ForwardCurve`.
pub trait Forward: TermStructure {
    /// Simple forward rate starting at time `t`.
    fn rate(&self, t: f64) -> f64;

    /// Average rate over `[t1, t2]`.
    #[inline]
    fn rate_period(&self, t1: f64, t2: f64) -> f64 {
        debug_assert!(t2 > t1, "t2 must be after t1");
        (self.rate(t1) + self.rate(t2)) * 0.5
    }
}

/// Minimal trait for survival/hazard curve polymorphism where needed.
/// Most code should call methods directly on `HazardCurve`.
pub trait Survival: TermStructure {
    /// Survival probability up to time `t`.
    fn sp(&self, t: f64) -> f64;
}

/// Minimal trait for term structure polymorphism where needed.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::traits::TermStructure;
/// use finstack_core::types::CurveId;
///
/// struct DummyCurve { id: CurveId }
///
/// impl TermStructure for DummyCurve {
///     fn id(&self) -> &CurveId { &self.id }
/// }
///
/// let curve = DummyCurve { id: CurveId::from("DUMMY") };
/// assert_eq!(curve.id().as_str(), "DUMMY");
/// ```
pub trait TermStructure {
    /// Unique identifier of the term structure.
    fn id(&self) -> &crate::types::CurveId;
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::types::CurveId;

    struct FlatCurve {
        id: CurveId,
        df_const: f64,
    }

    impl FlatCurve {
        fn new(id: &'static str, df_const: f64) -> Self {
            Self {
                id: CurveId::new(id),
                df_const,
            }
        }
    }

    impl TermStructure for FlatCurve {
        fn id(&self) -> &crate::types::CurveId {
            &self.id
        }
    }

    impl Discounting for FlatCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid test date")
        }
        fn df(&self, _t: f64) -> f64 {
            self.df_const
        }
    }

    #[test]
    fn discounting_trait_works() {
        let c = FlatCurve::new("TEST", 0.9);
        let df = c.df(1.0);
        assert_eq!(df, 0.9);
    }

    #[test]
    fn discounting_df_between_dates_constant_curve_is_one() {
        let c = FlatCurve::new("TEST", 0.9);
        let as_of = c.base_date();
        let to = as_of + time::Duration::days(365);
        let df = c
            .df_between_dates(as_of, to)
            .expect("constant curve should produce valid DFs");
        assert_eq!(df, 1.0);
    }

    #[test]
    fn discounting_df_between_times_validates_denominator() {
        let c = FlatCurve::new("TEST", 0.0);
        assert!(c.df_between_times(0.0, 1.0).is_err());
    }
}

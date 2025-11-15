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

use crate::dates::Date;

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
/// use time::Month;
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
///         Date::from_calendar_date(2025, Month::January, 1).unwrap()
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
            Date::from_calendar_date(2025, time::Month::January, 1)
                .expect("Valid test date")
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
}

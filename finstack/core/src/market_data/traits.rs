//! New trait family replacing legacy `Curve` hierarchy

use crate::{dates::Date, market_data::id::CurveId, F};
extern crate alloc;
#[cfg(feature = "parallel")]
#[allow(unused_imports)]
use rayon::prelude::*;

// -----------------------------------------------------------------------------
// Core super-trait
// -----------------------------------------------------------------------------
/// Common super-trait for all term-structure types (curves, surfaces, lattices).
///
/// The trait purposefully only exposes the [`id`] accessor which every market
/// object shares.  Specialised behaviour lives in more focused traits such as
/// [`DiscountCurve`], [`ForwardCurve`], [`SurvivalCurve`], [`PriceIndexCurve`]
/// and [`Surface`].  For the time being the legacy [`Curve`] implementation is
/// kept as a convenience wrapper that extends `TermStructure` so downstream
/// code continues to compile while the refactor is rolled out incrementally.
pub trait TermStructure {
    /// Unique identifier of the term structure (e.g. "USD-OIS").
    fn id(&self) -> &CurveId;
}

// -----------------------------------------------------------------------------
// Specialised traits
// -----------------------------------------------------------------------------

/// Discount-factor based curve.
pub trait Discount: TermStructure {
    /// Base (valuation) date of the curve.
    fn base_date(&self) -> Date;
    /// Discount factor at time `t` (year fraction from the base date).
    fn df(&self, t: F) -> F;

    /// Continuously-compounded zero rate.
    #[inline]
    fn zero(&self, t: F) -> F {
        if t == 0.0 {
            return 0.0;
        }
        -self.df(t).ln() / t
    }

    /// Simple forward rate between `t1` and `t2`.
    #[inline]
    fn fwd(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 > t1, "fwd requires t2 > t1");
        let z1 = self.zero(t1) * t1;
        let z2 = self.zero(t2) * t2;
        (z1 - z2) / (t2 - t1)
    }

    /// Batch evaluation helper (parallel over `times` slice when compiled
    /// with the `parallel` feature).
    #[cfg_attr(docsrs, doc(cfg(feature = "parallel")))]
    fn df_batch(&self, times: &[F]) -> alloc::vec::Vec<F>
    where
        Self: Sync,
    {
        #[cfg(all(feature = "parallel", not(feature = "deterministic")))]
        {
            times.par_iter().map(|&t| self.df(t)).collect()
        }
        #[cfg(any(not(feature = "parallel"), feature = "deterministic"))]
        {
            times.iter().map(|&t| self.df(t)).collect()
        }
    }
}

/// Forward-rate curves (e.g. 3-month SOFR).
pub trait Forward: TermStructure {
    /// Simple forward rate starting at time `t`.
    fn rate(&self, t: F) -> F;

    /// Average rate over `[t1, t2]`.
    #[inline]
    fn rate_period(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 > t1, "t2 must be after t1");
        (self.rate(t1) + self.rate(t2)) * 0.5
    }
}

/// Credit survival / hazard curves.
pub trait Survival: TermStructure {
    /// Survival probability up to time `t`.
    fn sp(&self, t: F) -> F;

    /// Default probability between `t1` and `t2`.
    #[inline]
    fn default_prob(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 >= t1, "t2 must be >= t1");
        let s1 = self.sp(t1);
        let s2 = self.sp(t2);
        s1 - s2
    }
}

/// CPI / real inflation index curves.
pub trait Inflation: TermStructure {
    /// CPI at time `t`.
    fn cpi(&self, t: F) -> F;

    /// Simple annualised inflation rate.
    #[inline]
    fn inflation_rate(&self, t1: F, t2: F) -> F {
        debug_assert!(t2 > t1, "t2 must be after t1");
        (self.cpi(t2) / self.cpi(t1) - 1.0) / (t2 - t1)
    }
}

/// Generic 2-D surface (e.g. volatility grid).
pub trait Surface: TermStructure {
    /// Surface value at (`x`, `y`).
    fn value(&self, x: F, y: F) -> F;
}

// -----------------------------------------------------------------------------
// Tests – confirm helper methods behave as expected
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    struct FlatCurve {
        id: CurveId,
        df_const: F,
    }

    impl FlatCurve {
        const fn new(id: &'static str, df_const: F) -> Self {
            Self {
                id: CurveId::new(id),
                df_const,
            }
        }
    }

    impl TermStructure for FlatCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discount for FlatCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, time::Month::January, 1).unwrap()
        }
        fn df(&self, _t: F) -> F {
            self.df_const
        }
    }

    #[test]
    fn zero_rate_matches_formula() {
        let c = FlatCurve::new("TEST", 0.9);
        let t = 2.0;
        let expected = -0.9f64.ln() / t;
        assert!((c.zero(t) - expected).abs() < 1e-12);
    }

    #[test]
    fn fwd_rate_matches_difference() {
        let c = FlatCurve::new("TEST", 0.95);
        let (t1, t2) = (1.0, 3.0);
        let fwd = c.fwd(t1, t2);
        assert!(fwd.abs() < 1e-12);
    }
}

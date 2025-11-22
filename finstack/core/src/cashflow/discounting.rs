//! Present value calculations using market discount curves.
//!
//! This module provides functions for discounting cashflows using market-derived
//! discount curves rather than constant rates. This is the standard approach for
//! pricing fixed income securities and derivatives.
//!
//! # Approach
//!
//! Unlike constant-rate discounting (see [`performance`](super::performance)),
//! this module uses term structures of discount factors from market data:
//! ```text
//! PV = Σ CF_i * DF(t_i)
//!
//! where DF(t) is the discount factor from the market curve
//! ```
//!
//! # Use Cases
//!
//! - **Bond pricing**: Government and corporate bonds
//! - **Swap valuation**: Interest rate swaps using OIS/LIBOR curves
//! - **Derivative pricing**: Future cashflows under risk-neutral measure
//! - **Portfolio valuation**: Mark-to-market of fixed income positions
//!
//! # Examples
//!
//! ```rust
//! use finstack_core::cashflow::discounting::npv_static;
//! use finstack_core::market_data::term_structures::DiscountCurve;
//! use finstack_core::dates::{Date, DayCount};
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use time::Month;
//!
//! // Build a flat discount curve
//! let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(base_date)
//!     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
//!     .build()?;
//!
//! // Cashflows to discount
//! let cf1 = (
//!     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
//!     Money::new(100.0, Currency::USD)
//! );
//! let flows = vec![cf1];
//!
//! let pv = npv_static(&curve, base_date, DayCount::Act360, &flows)?;
//! assert!(pv.amount() < 100.0); // Discounted value < face value
//! # Ok::<(), finstack_core::Error>(())
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Pearson. Chapters 4-7 (Interest Rates and Curve Construction).
//! - Andersen, L., & Piterbarg, V. (2010). *Interest Rate Modeling* (3 vols).
//!   Atlantic Financial Press. Volume 1, Chapter 3.

use crate::cashflow::utils::signed_year_fraction;
use crate::dates::{Date, DayCount, DayCountCtx};
use crate::market_data::traits::Discounting;
use crate::money::Money;

/// Objects that can be present-valued against a `Discount` curve.
///
/// Provides a unified interface for NPV calculations across different
/// cashflow representations and instrument types. Implemented for any
/// type that implements `AsRef<[(Date, Money)]>` (including `&[(..)]`
/// and `Vec<(..)>`).
pub trait Discountable {
    /// Output type for the NPV calculation.
    type PVOutput;

    /// Compute present value using the given discount curve and day count.
    fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> Self::PVOutput;
}

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount` with static dispatch.
///
/// This generic helper avoids dynamic dispatch on the discount curve in tight loops.
pub fn npv_static<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    if flows.is_empty() {
        return Err(crate::error::InputError::TooFewPoints.into());
    }
    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);
    let ctx = DayCountCtx::default();
    for (d, amt) in flows {
        let t = signed_year_fraction(dc, base, *d, ctx)?;
        let df = disc.df(t);
        let disc_amt = *amt * df;
        total = (total + disc_amt)?;
    }
    Ok(total)
}

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
///
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
impl<T> Discountable for T
where
    T: AsRef<[(Date, Money)]>,
{
    type PVOutput = crate::Result<Money>;

    fn npv(&self, disc: &dyn Discounting, base: Date, dc: DayCount) -> crate::Result<Money> {
        npv_static(disc, base, dc, self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::traits::TermStructure;
    use crate::types::CurveId;
    use time::Month;

    struct FlatCurve {
        id: CurveId,
    }

    impl TermStructure for FlatCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for FlatCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
        }
        fn df(&self, _t: f64) -> f64 {
            1.0
        }
    }

    #[test]
    fn tuples_discountable_paths_through() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let flows = vec![
            (base, Money::new(10.0, crate::currency::Currency::USD)),
            (base, Money::new(5.0, crate::currency::Currency::USD)),
        ];
        let pv = flows
            .npv(&curve, base, DayCount::Act365F)
            .expect("NPV calculation should succeed in test");
        assert!((pv.amount() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn npv_errors_on_empty_flows() {
        let curve = FlatCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let flows: Vec<(Date, Money)> = vec![];
        let err = npv_static(&curve, base, DayCount::Act365F, &flows)
            .expect_err("Should fail with empty flows");
        let _ = format!("{}", err);
    }
}

//! Present value calculations using market discount curves.
//!
//! This module provides functions for discounting cashflows using market-derived
//! discount curves rather than constant rates. This is the standard approach for
//! pricing fixed income securities and derivatives.
//!
//! # Approach
//!
//! Unlike constant-rate discounting (see [`xirr`](super::xirr)),
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
//! use finstack_core::cashflow::discounting::npv;
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
//! let pv = npv(&curve, base_date, DayCount::Act360, &flows)?;
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

use crate::dates::{Date, DayCount, DayCountCtx};
use crate::market_data::term_structures::FlatCurve;
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
///
/// **Note**: For consistent pricing with metrics (e.g., par rate), prefer using
/// [`npv_using_curve_dc`] which uses the curve's own day count convention.
pub fn npv<D: Discounting + ?Sized>(
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
        let t = dc.signed_year_fraction(base, *d, ctx)?;
        let df = disc.df(t);
        let disc_amt = *amt * df;
        total = (total + disc_amt)?;
    }
    Ok(total)
}

/// Compute NPV of dated `Money` flows using the curve's own day count convention.
///
/// Unlike [`npv`], this function uses the curve's internal day count
/// for computing year fractions. This ensures consistency between:
/// - Metric calculations (e.g., par rate which uses `df_on_date_curve`)
/// - NPV calculations
///
/// **Use this function when pricing instruments at par rate should yield zero PV.**
///
/// # Arguments
///
/// * `disc` - Discount curve implementing the `Discounting` trait
/// * `base` - Valuation date (flows before this are ignored)
/// * `flows` - Dated cashflows to discount
///
/// # Example
///
/// ```rust
/// use finstack_core::cashflow::discounting::npv_using_curve_dc;
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::dates::{Date, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
///
/// let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base_date)
///     .day_count(DayCount::Act360) // Curve's day count
///     .knots([(0.0, 1.0), (1.0, 0.95)])
///     .build()?;
///
/// let cf = (
///     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
///     Money::new(100.0, Currency::USD)
/// );
/// let flows = vec![cf];
///
/// // Uses curve's Act360 day count for year fraction calculation
/// let pv = npv_using_curve_dc(&curve, base_date, &flows)?;
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn npv_using_curve_dc<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    npv(disc, base, disc.day_count(), flows)
}

/// Calculate Net Present Value (NPV) using a constant discount rate.
///
/// This convenience function creates a flat discount curve internally and
/// delegates to the standard discounting logic.
///
/// # Arguments
/// * `cash_flows` - Vector of (date, money) tuples
/// * `discount_rate` - Annual discount rate as decimal (0.05 = 5%)
/// * `base_date` - Base date for discounting
/// * `day_count` - Day count convention
///
/// # Example
///
/// ```rust
/// use finstack_core::cashflow::discounting::npv_constant;
/// use finstack_core::dates::{Date, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let cf = (
///     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
///     Money::new(105.0, Currency::USD)
/// );
///
/// // Discount at 5%
/// let pv = npv_constant(&[cf], 0.05, base, DayCount::Act365F)?;
/// assert!((pv.amount() - 100.0).abs() < 0.1);
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn npv_constant(
    cash_flows: &[(Date, Money)],
    discount_rate: f64,
    base_date: Date,
    day_count: DayCount,
) -> crate::Result<Money> {
    // Convert annual compounded rate to continuous for FlatCurve
    // Note: FlatCurve expects a continuously compounded rate.
    // Ideally we should clarify if input is annually compounded or continuous.
    // Legacy behavior in performance.rs assumed input was annually compounded
    // and converted it: r_cont = ln(1 + r_annual).
    let continuous_rate = (1.0 + discount_rate).ln();
    let curve = FlatCurve::new(continuous_rate, base_date, day_count, "INTERNAL-NPV");

    // Delegate to the shared discounting logic
    npv(&curve, base_date, day_count, cash_flows)
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
        npv(disc, base, dc, self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use crate::dates::create_date;
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
        let err =
            npv(&curve, base, DayCount::Act365F, &flows).expect_err("Should fail with empty flows");
        let _ = format!("{}", err);
    }

    #[test]
    fn test_npv_simple() {
        let flows = vec![
            (
                create_date(2024, Month::January, 1).expect("Valid test date"),
                Money::new(-100000.0, Currency::USD),
            ),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                Money::new(110000.0, Currency::USD),
            ),
        ];
        let base = flows[0].0;
        let npv_5pct = npv_constant(&flows, 0.05, base, DayCount::Act365F)
            .expect("NPV calculation should succeed in test");
        // NPV should be positive (profitable at 5% discount rate)
        // Approximately: -100000 + 110000/(1.05) ≈ 4761.90
        assert!(npv_5pct.amount() > 4700.0 && npv_5pct.amount() < 4800.0);
    }

    #[test]
    fn test_npv_zero_discount() {
        let flows = vec![
            (
                create_date(2024, Month::January, 1).expect("Valid test date"),
                Money::new(-100.0, Currency::USD),
            ),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                Money::new(100.0, Currency::USD),
            ),
        ];
        let base = flows[0].0;
        let npv_zero = npv_constant(&flows, 0.0, base, DayCount::Act365F)
            .expect("NPV calculation should succeed in test");
        assert_eq!(npv_zero.amount(), 0.0);
    }

    #[test]
    fn test_npv_allows_past_and_future_dates() {
        let base = create_date(2025, Month::January, 1).expect("Valid test date");
        let flows = vec![
            (
                create_date(2024, Month::July, 1).expect("Valid test date"),
                Money::new(-50.0, Currency::USD),
            ), // past relative to base
            (
                create_date(2025, Month::July, 1).expect("Valid test date"),
                Money::new(55.0, Currency::USD),
            ), // future relative to base
        ];
        // Should not error; just compute signed year fractions
        let pv = npv_constant(&flows, 0.05, base, DayCount::Act365F)
            .expect("NPV calculation should succeed in test");
        // With positive rate and inflow slightly bigger than outflow, PV should be > 0
        assert!(pv.amount() > 0.0);
    }

    #[test]
    fn test_npv_errors_on_empty_flows_now() {
        let flows: Vec<(Date, Money)> = vec![];
        let base = create_date(2025, Month::January, 1).expect("Valid date");
        let err = npv_constant(&flows, 0.05, base, DayCount::Act365F)
            .expect_err("Should fail with empty flows");
        let _ = format!("{}", err);
    }
}

//! Repo pricing engine and facade.
//!
//! Provides deterministic valuation for repurchase agreements (repos),
//! separating pricing policy from instrument data structures.
//! The engine currently supports present value (NPV) from the
//! perspective of the cash lender (repo buyer):
//!
//! NPV = PV(total_repayment) - initial_cash_outflow
//!
//! where total_repayment = principal + interest, and discounting is
//! performed off the configured discount curve.

use crate::instruments::repo::Repo;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;

/// Pricing facade for `Repo` instruments.
#[derive(Clone, Debug, Default)]
pub struct RepoPricer;

impl RepoPricer {
    /// Create a new repo pricer with default configuration.
    pub fn new() -> Self {
        Self
    }

    /// Compute present value of the repo at `as_of` using curves in `context`.
    ///
    /// This delegates day-count and cashflow math to `Repo` helpers while
    /// centralizing curve access and discounting in one place.
    pub fn pv(&self, repo: &Repo, context: &MarketContext, _as_of: Date) -> Result<Money> {
        let disc_curve = context.get_discount_ref(repo.disc_id)?;

        // Total repayment at maturity (principal + interest)
        let total_repayment = repo.total_repayment()?;

        // Discount factors computed on the curve's own base-date time basis
        let base = disc_curve.base_date();
        let disc_dyn: &dyn finstack_core::market_data::traits::Discounting = disc_curve;
        let df_maturity = DiscountCurve::df_on(disc_dyn, base, repo.maturity, repo.day_count);
        let df_start = DiscountCurve::df_on(disc_dyn, base, repo.start_date, repo.day_count);

        // Present value of inflow at maturity minus PV of initial cash outflow at start
        let pv_in = total_repayment * df_maturity;
        let pv_out = repo.cash_amount * df_start;
        pv_in.checked_sub(pv_out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::instruments::repo::{CollateralSpec, Repo};
    use finstack_core::currency::Currency;
    use finstack_core::market_data::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    fn flat_discount(
        id: &'static str,
        base: Date,
    ) -> finstack_core::market_data::term_structures::discount_curve::DiscountCurve {
        finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(id)
            .base_date(base)
            .knots(vec![(0.0, 1.0), (10.0, 1.0)])
            .build()
            .unwrap()
    }

    fn sample_repo(start: Date, maturity: Date) -> Repo {
        let collateral = CollateralSpec::new("BOND_X", 1000.0, "BOND_X_PX");
        Repo::term(
            "R1",
            Money::new(1_000_000.0, Currency::USD),
            collateral,
            0.06,
            start,
            maturity,
            "USD-OIS",
        )
    }

    #[test]
    fn pv_equals_interest_when_df_is_one_and_start_is_base() {
        let base = date(2025, 1, 1);
        let start = base;
        let maturity = date(2025, 4, 1);
        let repo = sample_repo(start, maturity);

        // Flat DF=1 curve → PV should equal interest amount
        let disc = flat_discount("USD-OIS", base);
        let ctx = MarketContext::new().insert_discount(disc).insert_price(
            "BOND_X_PX",
            finstack_core::market_data::scalars::MarketScalar::Price(Money::new(
                1.0,
                Currency::USD,
            )),
        );

        let pv = repo.value(&ctx, base).unwrap();
        let interest = repo.interest_amount().unwrap();
        assert!((pv.amount() - interest.amount()).abs() < 1e-8);
        assert_eq!(pv.currency(), Currency::USD);
    }
}

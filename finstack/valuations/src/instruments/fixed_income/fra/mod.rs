//! Forward Rate Agreement (FRA) instrument implementation.
//!
//! FRAs are essential for short-end interest rate curve calibration,
//! providing forward rate fixings between deposit maturities and swap start dates.

pub mod metrics;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
#[derive(Clone, Debug)]
pub struct ForwardRateAgreement {
    /// Unique identifier
    pub id: String,
    /// Notional amount
    pub notional: Money,
    /// Rate fixing date (start of interest period)
    pub fixing_date: Date,
    /// Interest period start date
    pub start_date: Date,
    /// Interest period end date
    pub end_date: Date,
    /// Fixed rate (decimal, e.g., 0.05 for 5%)
    pub fixed_rate: F,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Reset lag in business days (fixing to value date)
    pub reset_lag: i32,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Pay/receive flag (true = receive fixed, pay floating)
    pub pay_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl ForwardRateAgreement {
    /// Create a new FRA.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        notional: Money,
        fixing_date: Date,
        start_date: Date,
        end_date: Date,
        fixed_rate: F,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional,
            fixing_date,
            start_date,
            end_date,
            fixed_rate,
            day_count,
            reset_lag: 2, // Standard T+2 settlement
            disc_id,
            forward_id,
            pay_fixed: false, // Default to receive fixed
            attributes: Attributes::new(),
        }
    }

    /// Set pay/receive direction.
    pub fn with_pay_fixed(mut self, pay_fixed: bool) -> Self {
        self.pay_fixed = pay_fixed;
        self
    }

    /// Set reset lag.
    pub fn with_reset_lag(mut self, reset_lag: i32) -> Self {
        self.reset_lag = reset_lag;
        self
    }

    /// Calculate FRA value using market curves.
    pub fn fra_value(
        &self,
        discount_curve: &dyn Discount,
        forward_curve: &dyn Forward,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Calculate time fractions
        let base_date = discount_curve.base_date();
        let t_fixing = DiscountCurve::year_fraction(base_date, self.fixing_date, self.day_count);
        let t_start = DiscountCurve::year_fraction(base_date, self.start_date, self.day_count);
        let t_end = DiscountCurve::year_fraction(base_date, self.end_date, self.day_count);

        // Interest period length
        let tau = DiscountCurve::year_fraction(self.start_date, self.end_date, self.day_count);

        if tau <= 0.0 || t_fixing < 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Get forward rate for the period
        let forward_rate = forward_curve.rate_period(t_start, t_end);

        // Get discount factor to settlement date (start of interest period for FRAs)
        let df_settlement = discount_curve.df(t_start);

        // FRA payoff: (Forward - Fixed) * tau * Notional * DF
        // Discounted to start of period (FRA convention)
        let rate_diff = forward_rate - self.fixed_rate;
        let pv = self.notional.amount() * rate_diff * tau * df_settlement;

        // Apply pay/receive direction
        let signed_pv = if self.pay_fixed { -pv } else { pv };

        Ok(Money::new(signed_pv, self.notional.currency()))
    }
}

impl_instrument!(
    ForwardRateAgreement,
    "FRA",
    pv = |s, curves, as_of| {
        let discount_curve = curves.discount(s.disc_id)?;
        let forward_curve = curves.forecast(s.forward_id)?;
        s.fra_value(discount_curve.as_ref(), forward_curve.as_ref(), as_of)
    }
);

impl CashflowProvider for ForwardRateAgreement {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // FRA settlement is at start of interest period
        if self.start_date <= as_of {
            return Ok(vec![]); // Already settled
        }

        // Calculate the FRA settlement amount
        let pv = self.fra_value(
            curves.discount(self.disc_id)?.as_ref(),
            curves.forecast(self.forward_id)?.as_ref(),
            as_of,
        )?;

        Ok(vec![(self.start_date, pv)])
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, forward_curve::ForwardCurve,
    };
    use time::Month;

    fn create_test_curves() -> (DiscountCurve, ForwardCurve) {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([
                (0.0, 1.0),
                (0.25, 0.988),
                (0.5, 0.975),
                (1.0, 0.95),
                (3.0, 0.90),
            ])
            .build()
            .unwrap();

        let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base_date)
            .knots([
                (0.0, 0.045),
                (0.25, 0.046),
                (0.5, 0.047),
                (1.0, 0.048),
                (3.0, 0.050),
            ])
            .build()
            .unwrap();

        (disc_curve, fwd_curve)
    }

    #[test]
    fn test_fra_creation() {
        let _base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let fixing_date = Date::from_calendar_date(2025, Month::June, 1).unwrap();
        let start_date = Date::from_calendar_date(2025, Month::June, 3).unwrap();
        let end_date = Date::from_calendar_date(2025, Month::September, 3).unwrap();

        let fra = ForwardRateAgreement::new(
            "6x9-FRA",
            Money::new(10_000_000.0, Currency::USD),
            fixing_date,
            start_date,
            end_date,
            0.045, // 4.5% fixed rate
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR-3M",
        );

        assert_eq!(fra.id, "6x9-FRA");
        assert_eq!(fra.fixed_rate, 0.045);
        assert!(!fra.pay_fixed); // Default is receive fixed
    }

    #[test]
    fn test_fra_valuation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let fixing_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let start_date = Date::from_calendar_date(2025, Month::March, 3).unwrap();
        let end_date = Date::from_calendar_date(2025, Month::June, 3).unwrap();

        let fra = ForwardRateAgreement::new(
            "3x6-FRA",
            Money::new(1_000_000.0, Currency::USD),
            fixing_date,
            start_date,
            end_date,
            0.045,
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR-3M",
        );

        let (disc_curve, fwd_curve) = create_test_curves();
        let pv = fra.fra_value(&disc_curve, &fwd_curve, base_date).unwrap();

        // Should have some value when forward ≠ fixed
        assert_ne!(pv.amount(), 0.0);
        assert_eq!(pv.currency(), Currency::USD);
    }

}

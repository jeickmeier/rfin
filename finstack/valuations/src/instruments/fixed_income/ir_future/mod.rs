//! Interest Rate Future instrument implementation.
//!
//! Represents exchange-traded interest rate futures like SOFR, Eurodollar,
//! or Short Sterling futures. Essential for calibrating the short end of
//! forward curves with proper convexity adjustments.

pub mod metrics;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

/// Interest Rate Future instrument.
#[derive(Clone, Debug)]
pub struct InterestRateFuture {
    /// Unique identifier
    pub id: String,
    /// Contract notional amount
    pub notional: Money,
    /// Future expiry/delivery date
    pub expiry_date: Date,
    /// Underlying rate fixing date
    pub fixing_date: Date,
    /// Rate period start date
    pub period_start: Date,
    /// Rate period end date
    pub period_end: Date,
    /// Quoted future price (e.g., 99.25)
    pub quoted_price: F,
    /// Day count convention
    pub day_count: DayCount,
    /// Contract specifications
    pub contract_specs: FutureContractSpecs,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Attributes
    pub attributes: Attributes,
}

/// Contract specifications for interest rate futures.
#[derive(Clone, Debug)]
pub struct FutureContractSpecs {
    /// Face value of contract
    pub face_value: F,
    /// Tick size
    pub tick_size: F,
    /// Tick value in currency units
    pub tick_value: F,
    /// Number of delivery months
    pub delivery_months: u8,
    /// Convexity adjustment (for long-dated contracts)
    pub convexity_adjustment: Option<F>,
}

impl Default for FutureContractSpecs {
    fn default() -> Self {
        Self {
            face_value: 1_000_000.0,
            tick_size: 0.0025, // 0.25 bp
            tick_value: 25.0,  // $25 per tick for $1MM
            delivery_months: 3,
            convexity_adjustment: None,
        }
    }
}

impl InterestRateFuture {
    /// Create a new interest rate future.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        notional: Money,
        expiry_date: Date,
        fixing_date: Date,
        period_start: Date,
        period_end: Date,
        quoted_price: F,
        day_count: DayCount,
        disc_id: &'static str,
        forward_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            notional,
            expiry_date,
            fixing_date,
            period_start,
            period_end,
            quoted_price,
            day_count,
            contract_specs: FutureContractSpecs::default(),
            disc_id,
            forward_id,
            attributes: Attributes::new(),
        }
    }

    /// Set contract specifications.
    pub fn with_contract_specs(mut self, specs: FutureContractSpecs) -> Self {
        self.contract_specs = specs;
        self
    }

    /// Get implied rate from quoted price.
    pub fn implied_rate(&self) -> F {
        (100.0 - self.quoted_price) / 100.0
    }

    /// Calculate future value with convexity adjustment.
    pub fn future_value(
        &self,
        discount_curve: &dyn Discount,
        forward_curve: &dyn Forward,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let base_date = discount_curve.base_date();

        // Time to fixing and rate period
        let t_fixing = self.day_count.year_fraction(base_date, self.fixing_date, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
        let t_start = self.day_count.year_fraction(base_date, self.period_start, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
        let t_end = self.day_count.year_fraction(base_date, self.period_end, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);

        // Get forward rate for the underlying period
        let forward_rate = forward_curve.rate_period(t_start, t_end);

        // Apply convexity adjustment for long-dated futures
        let adjusted_rate = if let Some(convexity_adj) = self.contract_specs.convexity_adjustment {
            forward_rate + convexity_adj
        } else if t_fixing > 2.0 {
            // Approximate convexity adjustment for futures beyond 2 years
            let vol_estimate = 0.01; // 1% rate volatility assumption
            let convexity = 0.5 * vol_estimate * vol_estimate * t_fixing * t_fixing;
            forward_rate + convexity
        } else {
            forward_rate
        };

        // Future value = (Model Rate - Implied Rate) × Face Value × Period Length
        let implied_rate = self.implied_rate();
        let tau = self.day_count.year_fraction(self.period_start, self.period_end, finstack_core::dates::DayCountCtx::default()).unwrap_or(0.0);
        let rate_diff = adjusted_rate - implied_rate;

        let pv = rate_diff * self.contract_specs.face_value * tau;

        Ok(Money::new(pv, self.notional.currency()))
    }
}

impl_instrument!(
    InterestRateFuture,
    "InterestRateFuture",
    pv = |s, curves, as_of| {
        let discount_curve = curves.disc(s.disc_id)?;
        let forward_curve = curves.fwd(s.forward_id)?;
        s.future_value(discount_curve.as_ref(), forward_curve.as_ref(), as_of)
    }
);

impl CashflowProvider for InterestRateFuture {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // Futures settle daily (mark-to-market), but for simplicity
        // we'll return the final settlement at expiry
        if self.expiry_date <= as_of {
            return Ok(vec![]); // Already expired
        }

        let settlement_pv = self.future_value(
            curves.disc(self.disc_id)?.as_ref(),
            curves.fwd(self.forward_id)?.as_ref(),
            as_of,
        )?;

        Ok(vec![(self.expiry_date, settlement_pv)])
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
    fn test_future_creation() {
        let _base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let fixing = Date::from_calendar_date(2025, Month::March, 13).unwrap();
        let period_start = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::June, 15).unwrap();

        let future = InterestRateFuture::new(
            "SOFR-MAR25",
            Money::new(1_000_000.0, Currency::USD),
            expiry,
            fixing,
            period_start,
            period_end,
            99.25, // Price implies 0.75% rate
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR-3M",
        );

        assert_eq!(future.implied_rate(), 0.0075); // 100 - 99.25 = 0.75%
        assert_eq!(future.contract_specs.face_value, 1_000_000.0);
    }

    #[test]
    fn test_future_valuation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let fixing = Date::from_calendar_date(2025, Month::March, 13).unwrap();
        let period_start = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let period_end = Date::from_calendar_date(2025, Month::June, 15).unwrap();

        let future = InterestRateFuture::new(
            "SOFR-MAR25",
            Money::new(1_000_000.0, Currency::USD),
            expiry,
            fixing,
            period_start,
            period_end,
            99.25,
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR-3M",
        );

        let (disc_curve, fwd_curve) = create_test_curves();
        let pv = future
            .future_value(&disc_curve, &fwd_curve, base_date)
            .unwrap();

        // Should have some value based on forward vs implied rate difference
        assert!(pv.amount().is_finite());
        assert_eq!(pv.currency(), Currency::USD);
    }

    #[test]
    fn test_convexity_adjustment() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2027, Month::March, 15).unwrap(); // Long-dated
        let fixing = Date::from_calendar_date(2027, Month::March, 13).unwrap();
        let period_start = Date::from_calendar_date(2027, Month::March, 15).unwrap();
        let period_end = Date::from_calendar_date(2027, Month::June, 15).unwrap();

        let future = InterestRateFuture::new(
            "SOFR-MAR27",
            Money::new(1_000_000.0, Currency::USD),
            expiry,
            fixing,
            period_start,
            period_end,
            97.50, // Implies 2.5% rate
            DayCount::Act360,
            "USD-OIS",
            "USD-SOFR-3M",
        );

        let (disc_curve, fwd_curve) = create_test_curves();

        // Valuation should handle convexity adjustment for long-dated future
        let pv = future.future_value(&disc_curve, &fwd_curve, base_date);
        assert!(pv.is_ok());
    }
}

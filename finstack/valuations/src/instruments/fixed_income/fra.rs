//! Forward Rate Agreement (FRA) instrument implementation.
//!
//! FRAs are essential for short-end interest rate curve calibration,
//! providing forward rate fixings between deposit maturities and swap start dates.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::{Discount, Forward};
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

/// Interest Rate Future instrument.
///
/// Represents exchange-traded interest rate futures like SOFR, Eurodollar,
/// or Short Sterling futures. Essential for calibrating the short end of
/// forward curves with proper convexity adjustments.
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
            tick_value: 25.0,   // $25 per tick for $1MM
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
        let t_fixing = DiscountCurve::year_fraction(base_date, self.fixing_date, self.day_count);
        let t_start = DiscountCurve::year_fraction(base_date, self.period_start, self.day_count);
        let t_end = DiscountCurve::year_fraction(base_date, self.period_end, self.day_count);
        
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
        let tau = DiscountCurve::year_fraction(self.period_start, self.period_end, self.day_count);
        let rate_diff = adjusted_rate - implied_rate;
        
        let pv = rate_diff * self.contract_specs.face_value * tau;
        
        Ok(Money::new(pv, self.notional.currency()))
    }
}

impl_instrument!(
    InterestRateFuture,
    "InterestRateFuture",
    pv = |s, curves, as_of| {
        let discount_curve = curves.discount(s.disc_id)?;
        let forward_curve = curves.forecast(s.forward_id)?;
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
            curves.discount(self.disc_id)?.as_ref(),
            curves.forecast(self.forward_id)?.as_ref(),
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
            .knots([(0.0, 1.0), (0.25, 0.988), (0.5, 0.975), (1.0, 0.95), (3.0, 0.90)])
            .build()
            .unwrap();

        let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base_date)
            .knots([(0.0, 0.045), (0.25, 0.046), (0.5, 0.047), (1.0, 0.048), (3.0, 0.050)])
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
        let pv = future.future_value(&disc_curve, &fwd_curve, base_date).unwrap();

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

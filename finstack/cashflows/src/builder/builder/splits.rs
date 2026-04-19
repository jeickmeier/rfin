//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds/overrides a payment split (cash/PIK/split) over a window (PIK toggle support).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_payment_window(&mut self, start: Date, end: Date, split: CouponType) -> &mut Self {
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Convenience: payment split program with boundary dates (PIK toggle windows).
    ///
    /// Creates a payment profile where the coupon payment type (Cash, PIK, or Split)
    /// changes over time. Common for PIK toggle bonds and mezzanine loans where
    /// the borrower can elect to capitalize interest during specific periods.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and payment splits: `&[(end_date, split)]`
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, FixedCouponSpec, CouponType
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2030, Month::January, 1)?;
    ///
    /// // PIK toggle: 100% PIK for first 2 years, 50/50 split for next 2 years, then all cash
    /// let payment_steps = [
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, CouponType::PIK),
    ///     (Date::from_calendar_date(2029, Month::January, 1)?, CouponType::Split {
    ///         cash_pct: dec!(0.5),
    ///         pik_pct: dec!(0.5)
    ///     }),
    ///     (maturity, CouponType::Cash),
    /// ];
    ///
    /// let fixed_spec = FixedCouponSpec {
    ///     coupon_type: CouponType::Cash,  // Will be overridden by payment program
    ///     rate: dec!(0.10),  // 10% PIK toggle
    ///     freq: Tenor::semi_annual(),
    ///     dc: DayCount::Thirty360,
    ///     bdc: BusinessDayConvention::Following,
    ///     calendar_id: "weekends_only".to_string(),
    ///     end_of_month: false,
    ///     payment_lag_days: 0,
    ///     stub: StubKind::None,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(25_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(fixed_spec)
    ///     .payment_split_program(&payment_steps)
    ///     .build_with_curves(None)?;
    ///
    /// // Check that PIK flows increase outstanding balance
    /// let outstanding_path = schedule.outstanding_path_per_flow()?;
    /// assert!(outstanding_path.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - Periods not covered by steps default to `Cash`
    /// - Steps must be ordered by end date
    /// - Works with both fixed and floating coupons
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn payment_split_program(&mut self, steps: &[(Date, CouponType)]) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("payment_split_program")
        else {
            return self;
        };
        let mut prev = issue;
        for &(end, split) in steps {
            if prev < end {
                let _ = self.add_payment_window(prev, end, split);
            }
            prev = end;
        }
        if prev < maturity {
            let _ = self.add_payment_window(prev, maturity, CouponType::Cash);
        }
        self
    }
}

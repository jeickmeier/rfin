//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds a step-up coupon specification.
    ///
    /// A step-up coupon starts at an initial rate and steps to different rates
    /// on specified dates. The compiler translates this into per-period fixed
    /// coupon schedules with the appropriate rate for each period.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn step_up_cf(&mut self, spec: StepUpCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "step_up_cf",
            spec.schedule_params(),
            CouponSpec::StepUp {
                initial_rate: spec.initial_rate,
                step_schedule: spec.step_schedule,
            },
            spec.coupon_type,
        )
    }

    /// Convenience: fixed step-up program using boundary dates.
    ///
    /// Creates a series of fixed-rate coupon windows where the rate changes at
    /// specified boundary dates. Common for step-up bonds where the coupon rate
    /// increases over time to compensate for credit deterioration risk.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and rates: `&[(end_date, rate)]`
    /// * `schedule` - Common schedule parameters (frequency, day count, etc.)
    /// * `default_split` - Payment type (Cash, PIK, or Split) for all windows
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_cashflows::builder::{CashFlowSchedule, ScheduleParams, CouponType};
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1)?;
    ///
    /// // Step-up bond: 4% for first year, 5% for second year, 6% thereafter
    /// let steps = [
    ///     (Date::from_calendar_date(2026, Month::January, 1)?, 0.04),
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, 0.05),
    ///     (maturity, 0.06),
    /// ];
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_stepup(
    ///         &steps,
    ///         ScheduleParams::quarterly_act360(),
    ///         CouponType::Cash,
    ///     )
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// - Steps must be ordered by end date
    /// - If the last step doesn't reach maturity, the last rate is extended
    /// - All windows use the same schedule parameters
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_stepup(
        &mut self,
        steps: &[(Date, f64)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_stepup") else {
            return self;
        };
        let mut prev = issue;
        for &(end, rate) in steps {
            let _ = self.add_fixed_coupon_window(prev, end, rate, schedule.clone(), default_split);
            prev = end;
        }
        if prev != maturity {
            // If the last step didn't reach maturity, extend using last rate
            if let Some(&(_, rate)) = steps.last() {
                let _ = self.add_fixed_coupon_window(prev, maturity, rate, schedule, default_split);
            }
        }
        self
    }

    /// Convenience: floating margin step-up program.
    ///
    /// Creates a series of floating-rate coupon windows where the margin over
    /// the floating index changes at specified boundary dates. Common for loans
    /// where the credit spread increases over time.
    ///
    /// # Arguments
    ///
    /// * `steps` - Boundary dates and margins: `&[(end_date, margin_bps)]`
    /// * `base_spec` - Canonical floating coupon spec; step margins replace its
    ///   `rate_spec.spread_bp` for each window
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, CouponType, FloatingCouponSpec, FloatingRateSpec
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1)?;
    ///
    /// // Floating rate loan: SOFR + 200bps, stepping up to +300bps, then +400bps
    /// let steps = [
    ///     (Date::from_calendar_date(2026, Month::January, 1)?, 200.0),
    ///     (Date::from_calendar_date(2027, Month::January, 1)?, 300.0),
    ///     (maturity, 400.0),
    /// ];
    ///
    /// let base = FloatingCouponSpec {
    ///     coupon_type: CouponType::Cash,
    ///     rate_spec: FloatingRateSpec {
    ///         index_id: CurveId::new("USD-SOFR"),
    ///         spread_bp: dec!(0),  // Overridden by steps
    ///         gearing: dec!(1),
    ///         gearing_includes_spread: true,
    ///         index_floor_bp: None,
    ///         all_in_cap_bp: None,
    ///         all_in_floor_bp: None,
    ///         index_cap_bp: None,
    ///         reset_freq: Tenor::quarterly(),
    ///         reset_lag_days: 2,
    ///         dc: DayCount::Act360,
    ///         bdc: BusinessDayConvention::ModifiedFollowing,
    ///         calendar_id: "weekends_only".to_string(),
    ///         fixing_calendar_id: None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///         overnight_compounding: None,
    ///         overnight_basis: None,
    ///         fallback: Default::default(),
    ///     },
    ///     freq: Tenor::quarterly(),
    ///     stub: StubKind::ShortFront,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(5_000_000.0, Currency::USD), issue, maturity)
    ///     .float_margin_stepup(&steps, base)
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn float_margin_stepup(
        &mut self,
        steps: &[(Date, f64)],
        base_spec: FloatingCouponSpec,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("float_margin_stepup")
        else {
            return self;
        };
        let mut prev = issue;
        for &(end, margin_bp) in steps {
            debug_assert!(
                margin_bp.is_finite(),
                "float_margin_stepup: margin_bp is not finite ({margin_bp})"
            );
            let Some(margin_decimal) = self.decimal_from_f64_or_record_error(
                "float_margin_stepup",
                "margin_bp",
                margin_bp,
            ) else {
                return self;
            };
            let window_spec = Self::floating_spec_with_margin(&base_spec, margin_decimal);
            let _ = self.add_float_coupon_window(prev, end, window_spec);
            prev = end;
        }
        if prev != maturity {
            let mut margin_decimal = base_spec.rate_spec.spread_bp;
            if let Some(&(_, margin_bp)) = steps.last() {
                debug_assert!(
                    margin_bp.is_finite(),
                    "float_margin_stepup: last margin_bp is not finite ({margin_bp})"
                );
                let Some(last_margin_decimal) = self.decimal_from_f64_or_record_error(
                    "float_margin_stepup",
                    "margin_bp",
                    margin_bp,
                ) else {
                    return self;
                };
                margin_decimal = last_margin_decimal;
            }
            let _ = self.add_float_coupon_window(
                prev,
                maturity,
                Self::floating_spec_with_margin(&base_spec, margin_decimal),
            );
        }
        self
    }

    /// Convenience: fixed-to-float switch at `switch` date.
    ///
    /// Creates a hybrid instrument that pays fixed coupons until a switch date,
    /// then converts to floating coupons. Common for convertible/callable bonds
    /// and structured products with changing payment profiles.
    ///
    /// # Arguments
    ///
    /// * `switch` - Date when coupon switches from fixed to floating
    /// * `fixed_win` - Fixed rate and schedule for pre-switch period
    /// * `float_spec` - Canonical floating coupon spec for the post-switch period
    /// * `fixed_split` - Payment type for the fixed pre-switch period
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, CouponType, FixedWindow, FloatingCouponSpec, FloatingRateSpec,
    ///     ScheduleParams,
    /// };
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let issue = Date::from_calendar_date(2025, Month::January, 1)?;
    /// let switch = Date::from_calendar_date(2027, Month::January, 1)?;
    /// let maturity = Date::from_calendar_date(2030, Month::January, 1)?;
    ///
    /// // Pay 5% fixed for 2 years, then SOFR + 250bps floating
    /// let fixed_win = FixedWindow {
    ///     rate: dec!(0.05),
    ///     schedule: ScheduleParams::semiannual_30360(),
    /// };
    ///
    /// let float_spec = FloatingCouponSpec {
    ///     coupon_type: CouponType::Cash,
    ///     rate_spec: FloatingRateSpec {
    ///         index_id: CurveId::new("USD-SOFR"),
    ///         spread_bp: dec!(250),
    ///         gearing: dec!(1),
    ///         gearing_includes_spread: true,
    ///         index_floor_bp: None,
    ///         all_in_cap_bp: None,
    ///         all_in_floor_bp: None,
    ///         index_cap_bp: None,
    ///         reset_freq: Tenor::quarterly(),
    ///         reset_lag_days: 2,
    ///         dc: DayCount::Act360,
    ///         bdc: BusinessDayConvention::ModifiedFollowing,
    ///         calendar_id: "weekends_only".to_string(),
    ///         fixing_calendar_id: None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///         overnight_compounding: None,
    ///         overnight_basis: None,
    ///         fallback: Default::default(),
    ///     },
    ///     freq: Tenor::quarterly(),
    ///     stub: StubKind::ShortFront,
    /// };
    ///
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(10_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_to_float(switch, fixed_win, float_spec, CouponType::Cash)
    ///     .build_with_curves(None)?;
    ///
    /// assert!(schedule.flows.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_to_float(
        &mut self,
        switch: Date,
        fixed_win: FixedWindow,
        float_spec: FloatingCouponSpec,
        fixed_split: CouponType,
    ) -> &mut Self {
        let Some((issue, maturity)) = self.issue_maturity_or_record_error("fixed_to_float") else {
            return self;
        };
        let Some(rate_f64) = self.f64_from_decimal_or_record_error(
            "fixed_to_float",
            "fixed_win.rate",
            fixed_win.rate,
        ) else {
            return self;
        };
        let _ =
            self.add_fixed_coupon_window(issue, switch, rate_f64, fixed_win.schedule, fixed_split);
        let _ = self.add_float_coupon_window(switch, maturity, float_spec);
        self
    }
}

//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds a fixed coupon specification.
    ///
    /// The coupon leg spans the full principal horizon set by
    /// [`principal`](Self::principal). The builder emits fixed, stub, cash,
    /// split, or PIK coupon flows according to the supplied spec and the
    /// schedule conventions inside it.
    ///
    /// # Arguments
    ///
    /// * `spec` - Fixed-rate coupon quote, payment split, and schedule
    ///   conventions.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method records a deferred error if principal dates have not been
    /// set. Schedule generation, day-count, calendar, and coupon-split errors
    /// are returned by [`build_with_curves`](Self::build_with_curves) or
    /// [`prepared`](Self::prepared).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fixed_cf(FixedCouponSpec {
    ///         coupon_type: CouponType::Cash,
    ///         rate: dec!(0.05),
    ///         freq: Tenor::semi_annual(),
    ///         dc: DayCount::Thirty360,
    ///         bdc: BusinessDayConvention::Following,
    ///         calendar_id: "weekends_only".to_string(),
    ///         stub: StubKind::None,
    ///         end_of_month: false,
    ///         payment_lag_days: 0,
    ///     })
    ///     .build_with_curves(None)
    ///     .expect("fixed schedule builds");
    ///
    /// assert!(!schedule.flows.is_empty());
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "fixed_cf",
            spec.schedule_params(),
            CouponSpec::Fixed { rate: spec.rate },
            spec.coupon_type,
        )
    }

    /// Adds a floating coupon specification.
    ///
    /// The coupon leg spans the full principal horizon set by
    /// [`principal`](Self::principal). Floating-rate projection is deferred
    /// until [`build_with_curves`](Self::build_with_curves) or
    /// [`PreparedCashFlow::project`](super::PreparedCashFlow::project), where
    /// the forward curve or fallback policy is applied.
    ///
    /// # Arguments
    ///
    /// * `spec` - Floating-rate index, spread, caps/floors, payment split, and
    ///   schedule conventions.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method records a deferred error if principal dates have not been
    /// set. Floating spec validation, missing forward curves, calendar errors,
    /// and fallback-policy failures are returned by the terminal build or
    /// project step.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{
    ///     CashFlowSchedule, CouponType, FloatingCouponSpec, FloatingRateFallback,
    ///     FloatingRateSpec,
    /// };
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::CurveId;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .floating_cf(FloatingCouponSpec {
    ///         coupon_type: CouponType::Cash,
    ///         rate_spec: FloatingRateSpec {
    ///             index_id: CurveId::new("USD-SOFR-3M"),
    ///             spread_bp: dec!(200),
    ///             gearing: dec!(1),
    ///             gearing_includes_spread: true,
    ///             index_floor_bp: Some(dec!(0)),
    ///             all_in_floor_bp: None,
    ///             all_in_cap_bp: None,
    ///             index_cap_bp: None,
    ///             reset_freq: Tenor::quarterly(),
    ///             reset_lag_days: 2,
    ///             dc: DayCount::Act360,
    ///             bdc: BusinessDayConvention::ModifiedFollowing,
    ///             calendar_id: "weekends_only".to_string(),
    ///             fixing_calendar_id: None,
    ///             end_of_month: false,
    ///             payment_lag_days: 0,
    ///             overnight_compounding: None,
    ///             overnight_basis: None,
    ///             fallback: FloatingRateFallback::SpreadOnly,
    ///         },
    ///         freq: Tenor::quarterly(),
    ///         stub: StubKind::None,
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        self.push_full_horizon_coupon(
            "floating_cf",
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }

    /// Adds a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    ///
    /// Internal helper used by `fixed_stepup` / `fixed_to_float` etc. Prefer the
    /// spec-level entry points (`fixed_cf`, `fixed_stepup`, `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub(crate) fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: f64,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        debug_assert!(
            rate.is_finite(),
            "add_fixed_coupon_window: rate is not finite ({rate})"
        );
        let Some(rate_decimal) =
            self.decimal_from_f64_or_record_error("add_fixed_coupon_window", "rate", rate)
        else {
            return self;
        };
        self.push_coupon_window(
            start,
            end,
            schedule,
            CouponSpec::Fixed { rate: rate_decimal },
            split,
        )
    }

    /// Adds a floating coupon window with its own schedule and payment split.
    ///
    /// Internal helper used by `float_margin_stepup` / `fixed_to_float` etc.
    /// Prefer the spec-level entry points (`floating_cf`, `float_margin_stepup`,
    /// `fixed_to_float`).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub(crate) fn add_float_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        spec: FloatingCouponSpec,
    ) -> &mut Self {
        self.push_coupon_window(
            start,
            end,
            Self::schedule_from_floating_spec(&spec),
            CouponSpec::Float {
                rate_spec: spec.rate_spec,
            },
            spec.coupon_type,
        )
    }
}

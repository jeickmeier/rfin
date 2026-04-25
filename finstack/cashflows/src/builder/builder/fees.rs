//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Adds a fee specification.
    ///
    /// Fixed fees emit a one-time `Fee` cashflow on their configured date.
    /// Periodic basis-point fees generate a schedule over the principal horizon
    /// and accrue against the configured [`crate::builder::FeeBase`].
    ///
    /// # Arguments
    ///
    /// * `spec` - Fixed or periodic fee specification to add to the schedule.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method does not return errors directly. Missing principal dates,
    /// invalid fee schedules, calendar lookup failures, and currency mismatches
    /// are returned by [`build_with_curves`](Self::build_with_curves) or
    /// [`prepared`](Self::prepared).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{CashFlowSchedule, FeeBase, FeeSpec};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
    /// use finstack_core::money::Money;
    /// use rust_decimal_macros::dec;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    /// let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .fee(FeeSpec::PeriodicBps {
    ///         base: FeeBase::Drawn,
    ///         bps: dec!(25),
    ///         freq: Tenor::quarterly(),
    ///         dc: DayCount::Act360,
    ///         bdc: BusinessDayConvention::ModifiedFollowing,
    ///         calendar_id: "weekends_only".to_string(),
    ///         stub: StubKind::None,
    ///         accrual_basis: Default::default(),
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }
}

//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Sets principal details and instrument horizon.
    ///
    /// This must be called before full-horizon coupon helpers such as
    /// [`fixed_cf`](Self::fixed_cf), [`floating_cf`](Self::floating_cf), or
    /// [`step_up_cf`](Self::step_up_cf). Those helpers infer their start and
    /// end dates from this principal horizon.
    ///
    /// Calling this method clears any previously recorded sticky builder error.
    /// It does not clear coupons, fees, or principal events already pushed onto
    /// the builder, so prefer creating a fresh builder for a new instrument.
    ///
    /// # Arguments
    ///
    /// * `initial` - Initial outstanding principal and currency.
    /// * `issue_date` - Contract issue or funding date.
    /// * `maturity` - Contract maturity date.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::CashFlowSchedule;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use finstack_core::money::Money;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
    /// let maturity = Date::from_calendar_date(2030, Month::January, 15).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder.principal(Money::new(1_000_000.0, Currency::USD), issue, maturity);
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn principal(&mut self, initial: Money, issue_date: Date, maturity: Date) -> &mut Self {
        self.pending_error = None;
        self.notional = Some(Notional {
            initial,
            amort: AmortizationSpec::None,
        });
        self.issue = Some(issue_date);
        self.maturity = Some(maturity);
        self
    }

    /// Configures amortization on the current notional.
    ///
    /// The amortization rule is attached to the notional previously set by
    /// [`principal`](Self::principal). If no principal has been set, this method
    /// is a no-op; missing principal is reported later by
    /// [`build_with_curves`](Self::build_with_curves) or
    /// [`prepared`](Self::prepared).
    ///
    /// # Arguments
    ///
    /// * `spec` - Principal paydown rule to apply during schedule generation.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
    ///
    /// # Errors
    ///
    /// This method does not return errors directly. Validation failures such as
    /// mismatched amortization currency, increasing remaining-principal paths,
    /// or excessive custom principal are returned by the terminal build step.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::builder::{AmortizationSpec, CashFlowSchedule};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use finstack_core::money::Money;
    /// use time::Month;
    ///
    /// let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    /// let maturity = Date::from_calendar_date(2028, Month::January, 1).expect("valid date");
    /// let mut builder = CashFlowSchedule::builder();
    ///
    /// let _ = builder
    ///     .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    ///     .amortization(AmortizationSpec::LinearTo {
    ///         final_notional: Money::new(0.0, Currency::USD),
    ///     });
    /// ```
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn amortization(&mut self, spec: AmortizationSpec) -> &mut Self {
        if let Some(n) = &mut self.notional {
            n.amort = spec;
        }
        self
    }

    /// Adds custom principal events (draws/repays) that adjust outstanding balance.
    ///
    /// `delta` increases outstanding when positive and decreases when negative.
    /// `cash` is the actual cash leg (e.g., net of OID); if omitted, cash = delta.
    ///
    /// # Errors
    ///
    /// Records a pending error if any event has mismatched currencies between
    /// `delta` and `cash`. The error will be returned when `build_with_curves(...)` is called.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    #[deprecated(note = "use add_principal_event for the canonical principal-event builder path")]
    pub fn principal_events(&mut self, events: &[PrincipalEvent]) -> &mut Self {
        if self.pending_error.is_some() {
            return self;
        }
        for ev in events {
            if ev.cash.currency() != ev.delta.currency() {
                self.pending_error = Some(finstack_core::Error::CurrencyMismatch {
                    expected: ev.delta.currency(),
                    actual: ev.cash.currency(),
                });
                return self;
            }
        }
        self.principal_events.extend(events.iter().cloned());
        self
    }

    /// Adds a single principal event.
    ///
    /// # Errors
    ///
    /// Records a pending error if `cash` is provided with a different currency
    /// than `delta`. The error will be returned when `build_with_curves(...)` is called.
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn add_principal_event(
        &mut self,
        date: Date,
        delta: Money,
        cash: Option<Money>,
        kind: CFKind,
    ) -> &mut Self {
        if self.pending_error.is_some() {
            return self;
        }
        let cash_leg = cash.unwrap_or(delta);
        if cash_leg.currency() != delta.currency() {
            self.pending_error = Some(finstack_core::Error::CurrencyMismatch {
                expected: delta.currency(),
                actual: cash_leg.currency(),
            });
            return self;
        }
        self.principal_events.push(PrincipalEvent {
            date,
            delta,
            cash: cash_leg,
            kind,
        });
        self
    }
}

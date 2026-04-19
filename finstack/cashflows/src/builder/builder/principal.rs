//! Split from `builder.rs` for readability.

use super::*;

impl CashFlowBuilder {
    /// Sets principal details and instrument horizon.
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

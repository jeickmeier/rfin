//! Principal, amortization, and ad-hoc principal-event builder methods.

use finstack_core::dates::Date;
use finstack_core::money::Money;

use crate::builder::orchestrator::{CashFlowBuilder, PrincipalEvent};
use crate::builder::{AmortizationSpec, Notional};
use crate::primitives::CFKind;

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
    /// * `issue_date` - Start date for the instrument.
    /// * `maturity` - Final maturity date.
    ///
    /// # Returns
    ///
    /// Mutable builder reference for fluent chaining.
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
    /// [`build_with_curves`](Self::build_with_curves).
    #[must_use = "builder methods should be chained or terminated with .build_with_curves(...)"]
    pub fn amortization(&mut self, spec: AmortizationSpec) -> &mut Self {
        if let Some(n) = &mut self.notional {
            n.amort = spec;
        }
        self
    }

    /// Adds a single principal event.
    ///
    /// `delta` controls the outstanding balance change. The emitted cashflow
    /// sign is derived from `kind`: `CFKind::Amortization` emits `cash` as a
    /// positive repayment, while all other kinds emit `-cash` as a borrower
    /// draw/notional cashflow.
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

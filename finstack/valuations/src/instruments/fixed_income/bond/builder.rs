//! Bond builder for flexible construction.

use super::{Bond, CallPutSchedule};
use crate::cashflow::primitives::AmortizationSpec;
use crate::cashflow::builder::CashFlowSchedule;
use finstack_core::prelude::*;
use finstack_core::F;

/// Builder pattern for creating Bond instruments.
///
/// Supports both traditional bond specification and custom cashflow schedules.
///
/// # Example
///
/// See unit tests and `examples/` for usage.
#[derive(Default)]
pub struct BondBuilder {
    id: Option<String>,
    notional: Option<Money>,
    coupon: Option<F>,
    freq: Option<finstack_core::dates::Frequency>,
    dc: Option<DayCount>,
    issue: Option<Date>,
    maturity: Option<Date>,
    disc_id: Option<&'static str>,
    quoted_clean: Option<F>,
    call_put: Option<CallPutSchedule>,
    amortization: Option<AmortizationSpec>,
    custom_cashflows: Option<CashFlowSchedule>,
}

impl BondBuilder {
    /// Set the bond identifier.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the notional amount.
    pub fn notional(mut self, notional: Money) -> Self {
        self.notional = Some(notional);
        self
    }

    /// Set the coupon rate.
    pub fn coupon(mut self, coupon: F) -> Self {
        self.coupon = Some(coupon);
        self
    }

    /// Set the payment frequency.
    pub fn freq(mut self, freq: finstack_core::dates::Frequency) -> Self {
        self.freq = Some(freq);
        self
    }

    /// Set the day count convention.
    pub fn dc(mut self, dc: DayCount) -> Self {
        self.dc = Some(dc);
        self
    }

    /// Set the issue date.
    pub fn issue(mut self, issue: Date) -> Self {
        self.issue = Some(issue);
        self
    }

    /// Set the maturity date.
    pub fn maturity(mut self, maturity: Date) -> Self {
        self.maturity = Some(maturity);
        self
    }

    /// Set the discount curve identifier.
    pub fn disc_curve(mut self, disc_id: &'static str) -> Self {
        self.disc_id = Some(disc_id);
        self
    }

    /// Set the quoted clean price.
    pub fn quoted_clean(mut self, price: Option<F>) -> Self {
        self.quoted_clean = price;
        self
    }

    /// Set call/put schedule.
    pub fn call_put(mut self, schedule: CallPutSchedule) -> Self {
        self.call_put = Some(schedule);
        self
    }

    /// Set amortization specification.
    pub fn amortization(mut self, spec: AmortizationSpec) -> Self {
        self.amortization = Some(spec);
        self
    }

    /// Set custom cashflow schedule.
    ///
    /// When provided, this overrides coupon generation from the bond's
    /// coupon rate and amortization specifications.
    pub fn cashflows(mut self, schedule: CashFlowSchedule) -> Self {
        // Extract some parameters from the schedule if not already set
        if self.notional.is_none() {
            self.notional = Some(schedule.notional.initial);
        }
        if self.dc.is_none() {
            self.dc = Some(schedule.day_count);
        }

        // Extract issue and maturity dates if not set
        let dates = schedule.dates();
        if !dates.is_empty() {
            if self.issue.is_none() {
                self.issue = Some(dates[0]);
            }
            if self.maturity.is_none() && dates.len() > 1 {
                self.maturity = Some(*dates.last().unwrap());
            }
        }

        self.custom_cashflows = Some(schedule);
        self
    }

    /// Build the bond instance.
    pub fn build(self) -> finstack_core::Result<Bond> {
        // Required fields (or derive from custom cashflows)
        let id = self.id.ok_or(finstack_core::error::InputError::Invalid)?;
        let disc_id = self
            .disc_id
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Extract from custom cashflows if available and not set
        let (notional, dc, issue, maturity) = if let Some(ref custom) = self.custom_cashflows {
            let notional = self.notional.unwrap_or(custom.notional.initial);
            let dc = self.dc.unwrap_or(custom.day_count);

            let dates = custom.dates();
            if dates.len() < 2 {
                return Err(finstack_core::error::InputError::TooFewPoints.into());
            }
            let issue = self.issue.unwrap_or(dates[0]);
            let maturity = self.maturity.unwrap_or(*dates.last().unwrap());

            (notional, dc, issue, maturity)
        } else {
            // For traditional bonds, these are required
            let notional = self
                .notional
                .ok_or(finstack_core::error::InputError::Invalid)?;
            let dc = self.dc.ok_or(finstack_core::error::InputError::Invalid)?;
            let issue = self
                .issue
                .ok_or(finstack_core::error::InputError::Invalid)?;
            let maturity = self
                .maturity
                .ok_or(finstack_core::error::InputError::Invalid)?;

            (notional, dc, issue, maturity)
        };

        // Default values for optional fields
        let coupon = self.coupon.unwrap_or(0.0);
        let freq = self
            .freq
            .unwrap_or(finstack_core::dates::Frequency::semi_annual());

        Ok(Bond {
            id,
            notional,
            coupon,
            freq,
            dc,
            issue,
            maturity,
            disc_id,
            quoted_clean: self.quoted_clean,
            call_put: self.call_put,
            amortization: self.amortization,
            custom_cashflows: self.custom_cashflows,
            attributes: crate::traits::Attributes::new(),
        })
    }
}

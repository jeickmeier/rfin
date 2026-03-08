//! Accrued interest calculator for structured credit instruments.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::money::Money;
use finstack_core::Result;

/// Calculates accrued interest for structured credit instruments.
///
/// Accrued interest is the pro-rata interest that has accrued since the last
/// payment date. For structured credit, this is calculated per tranche based on:
/// - Days elapsed since last payment
/// - Days in the current period
/// - Current coupon rate (which may be floating)
///
/// # Formula
///
/// Accrued = (Days Elapsed / Days in Period) × Coupon Rate × Notional
///
pub struct AccruedCalculator;

impl MetricCalculator for AccruedCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Prefer interest-only flows from detailed tranche cashflows when available.
        // This avoids the systematic overstatement that occurs when principal payments
        // are mixed into the accrual calculation (common for amortizing tranches).
        if let Some(details) = context.detailed_tranche_cashflows.as_ref() {
            if !details.interest_flows.is_empty() {
                return self.accrued_from_interest_flows(&details.interest_flows, context);
            }
        }

        // Fallback: derive from aggregated cashflows (interest + principal combined).
        // This overstates accrued for amortizing structures but maintains backward
        // compatibility when detailed tranche flows are not available.
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        if let (Some((first_date, _)), Some((last_date, _))) = (flows.first(), flows.last()) {
            if context.as_of < *first_date || context.as_of >= *last_date {
                return Ok(0.0);
            }
        } else {
            return Ok(0.0);
        }

        let (last_payment, next_payment) = find_surrounding_dates(flows, context.as_of)?;

        let day_count = context.day_count.unwrap_or(DayCount::Act360);
        let accrual_fraction =
            day_count.year_fraction(last_payment, context.as_of, DayCountCtx::default())?;
        let period_fraction =
            day_count.year_fraction(last_payment, next_payment, DayCountCtx::default())?;

        if period_fraction == 0.0 {
            return Ok(0.0);
        }

        let period_interest = flows
            .iter()
            .find(|(d, _)| *d == next_payment)
            .map(|(_, m)| m.amount())
            .unwrap_or(0.0);

        let accrued = period_interest * (accrual_fraction / period_fraction);

        Ok(accrued)
    }
}

impl AccruedCalculator {
    fn accrued_from_interest_flows(
        &self,
        interest_flows: &[(Date, Money)],
        context: &MetricContext,
    ) -> Result<f64> {
        if interest_flows.is_empty() {
            return Ok(0.0);
        }

        if let (Some((first_date, _)), Some((last_date, _))) =
            (interest_flows.first(), interest_flows.last())
        {
            if context.as_of < *first_date || context.as_of >= *last_date {
                return Ok(0.0);
            }
        }

        let (last_payment, next_payment) = find_surrounding_dates(interest_flows, context.as_of)?;

        let day_count = context.day_count.unwrap_or(DayCount::Act360);
        let accrual_fraction =
            day_count.year_fraction(last_payment, context.as_of, DayCountCtx::default())?;
        let period_fraction =
            day_count.year_fraction(last_payment, next_payment, DayCountCtx::default())?;

        if period_fraction == 0.0 {
            return Ok(0.0);
        }

        let period_interest = interest_flows
            .iter()
            .find(|(d, _)| *d == next_payment)
            .map(|(_, m)| m.amount())
            .unwrap_or(0.0);

        Ok(period_interest * (accrual_fraction / period_fraction))
    }
}

/// Helper to find the payment dates surrounding as_of date.
fn find_surrounding_dates(flows: &[(Date, Money)], as_of: Date) -> Result<(Date, Date)> {
    // Find last payment before or on as_of
    let last = flows
        .iter()
        .filter(|(d, _)| *d <= as_of)
        .map(|(d, _)| *d)
        .max();

    // Find next payment after as_of
    let next = flows
        .iter()
        .filter(|(d, _)| *d > as_of)
        .map(|(d, _)| *d)
        .min();

    match (last, next) {
        (Some(l), Some(n)) => Ok((l, n)),
        _ => Err(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "accrual_period".to_string(),
            },
        )),
    }
}

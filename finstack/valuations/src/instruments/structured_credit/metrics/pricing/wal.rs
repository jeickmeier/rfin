//! WAL (Weighted Average Life) calculator for structured credit.

use crate::instruments::structured_credit::types::TrancheCashflows;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::Result;

/// Calculate tranche-specific WAL from a `TrancheCashflows`.
///
/// WAL measures the average time until principal is repaid, weighted by the
/// amount of principal. This is a critical metric for structured credit as it
/// captures the impact of prepayments, amortization, and defaults.
///
/// # Formula
///
/// WAL = Σ(Principal_i × Time_i) / Σ(Principal_i)
///
/// Where:
/// - Principal_i = principal payment at time i
/// - Time_i = years from valuation date to payment date i
pub fn calculate_tranche_wal(cashflows: &TrancheCashflows, as_of: Date) -> Result<f64> {
    let mut weighted_sum = 0.0;
    let mut total_principal = 0.0;

    for (date, amount) in &cashflows.principal_flows {
        if *date <= as_of {
            continue;
        }

        let years = finstack_core::dates::DayCount::Act365F
            .year_fraction(as_of, *date, finstack_core::dates::DayCountCtx::default())
            .unwrap_or(0.0);
        weighted_sum += amount.amount() * years;
        total_principal += amount.amount();
    }

    if total_principal > 0.0 {
        Ok(weighted_sum / total_principal)
    } else {
        Ok(0.0)
    }
}

/// Calculates WAL (Weighted Average Life) in years.
///
/// WAL measures the average time until principal is repaid, weighted by the
/// amount of principal. This is a critical metric for structured credit as it
/// captures the impact of prepayments, amortization, and defaults.
///
/// # Formula
///
/// WAL = Σ(Principal_i × Time_i) / Σ(Principal_i)
///
/// Where:
/// - Principal_i = principal payment at time i
/// - Time_i = years from valuation date to payment date i
///
/// # Market Conventions
///
/// - **CLO**: Typically 3-5 years
/// - **ABS**: Typically 2-4 years (varies with prepayment assumptions)
/// - **RMBS**: Typically 3-7 years (highly sensitive to PSA speed)
/// - **CMBS**: Typically 4-8 years
///
pub struct WalCalculator;

impl MetricCalculator for WalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        if let Some(details) = context.detailed_tranche_cashflows.as_ref() {
            return calculate_tranche_wal(details, context.as_of);
        }

        // Fallback: derive an approximate WAL directly from aggregated cashflows when
        // detailed tranche flows were not cached into the metric context. This keeps the
        // metric available for simple deals (e.g., single-tranche) without re-running the
        // structured credit simulation just to populate tranche-level cashflows.
        if let Some(flows) = context.cashflows.as_ref() {
            let mut weighted_sum = 0.0;
            let mut total_principal = 0.0;

            for (date, amount) in flows {
                if *date <= context.as_of {
                    continue;
                }

                let principal = amount.amount().abs();
                if principal == 0.0 {
                    continue;
                }

                let years = finstack_core::dates::DayCount::Act365F
                    .year_fraction(
                        context.as_of,
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                weighted_sum += principal * years;
                total_principal += principal;
            }

            return if total_principal > 0.0 {
                Ok(weighted_sum / total_principal)
            } else {
                Ok(0.0)
            };
        }

        Err(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "detailed_tranche_cashflows".to_string(),
            },
        ))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // No metric dependencies - uses cashflows from context
    }
}

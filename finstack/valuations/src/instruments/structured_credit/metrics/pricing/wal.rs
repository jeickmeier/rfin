//! WAL (Weighted Average Life) calculator for structured credit.

use crate::instruments::structured_credit::components::TrancheCashflowResult;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::Result;

/// Calculate tranche-specific WAL from a `TrancheCashflowResult`.
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
pub fn calculate_tranche_wal(cashflows: &TrancheCashflowResult, as_of: Date) -> Result<f64> {
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
        // Prefer the detailed tranche cashflow result if available for an exact WAL.
        if let Some(ref details) = context.detailed_tranche_cashflows {
            return calculate_tranche_wal(details, context.as_of);
        }

        // Fallback to legacy logic if detailed flows are not available.
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // Use Act/365F for year fraction calculation (common for WAL)
        let day_count = finstack_core::dates::DayCount::Act365F;

        let mut weighted_sum = 0.0;
        let mut total_principal = 0.0;

        // Identify principal payments and calculate weighted average
        for (date, amount) in flows {
            if *date <= context.as_of {
                continue; // Skip past cashflows
            }

            let amt = amount.amount();

            // For structured credit, we consider all positive cashflows as including principal
            // A more sophisticated implementation would tag cashflows by type
            // For now, we treat all flows as potentially containing principal
            if amt > 0.0 {
                let years = day_count
                    .year_fraction(context.as_of, *date, DayCountCtx::default())
                    .unwrap_or(0.0);

                weighted_sum += amt * years;
                total_principal += amt;
            }
        }

        // Calculate WAL
        if total_principal > 0.0 {
            Ok(weighted_sum / total_principal)
        } else {
            Ok(0.0)
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // No metric dependencies - uses cashflows from context
    }
}

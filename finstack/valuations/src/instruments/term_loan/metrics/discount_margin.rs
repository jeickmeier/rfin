//! Discount Margin for floating-rate term loans.
//!
//! Solves for an additive spread (decimal) to the loan's base spread such that
//! discounted PV of all cashflows (coupons + principal) matches observed price.
//!
//! # Market-Standard Implementation
//!
//! This implementation uses **full-fidelity re-pricing**:
//! - Clones the TermLoan and adjusts the spread by DM
//! - Re-runs complete cashflow generation including:
//!   * DDTL draw timing and fees
//!   * Amortization schedules
//!   * PIK capitalization
//!   * Cash sweeps and covenants
//!   * Principal redemptions
//! - Uses the actual pricer to compute PV
//! - Solves for DM using Brent's method
//!
//! This ensures DM is consistent with the loan's true cashflow structure.

use crate::instruments::TermLoan;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::math::solver::{BrentSolver, Solver};

/// Discount margin calculator for floating rate term loans
pub struct DiscountMarginCalculator;

impl DiscountMarginCalculator {
    /// Compute PV of term loan with adjusted spread (base_spread + dm_bp).
    ///
    /// Clones the loan, adds `dm_bp` to the floating spread, and re-prices using
    /// the full cashflow engine and pricer.
    fn pv_given_dm(
        loan: &TermLoan,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        dm_bp: f64,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::term_loan::types::RateSpec;

        // Clone loan and adjust spread
        let mut loan_with_dm = loan.clone();

        match &mut loan_with_dm.rate {
            RateSpec::Floating(spec) => {
                // Add DM (in bp) to base spread
                spec.spread_bp += dm_bp;
            }
            RateSpec::Fixed { .. } => {
                // Should not happen (caller checks), but return zero if called on fixed rate
                return Ok(0.0);
            }
        }

        // Re-price using full cashflow engine and pricer
        let pv = crate::instruments::term_loan::pricing::TermLoanDiscountingPricer::price(
            &loan_with_dm,
            curves,
            as_of,
        )?;

        Ok(pv.amount())
    }
}

impl MetricCalculator for DiscountMarginCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let loan: &TermLoan = context.instrument_as()?;

        // If not floating, DM = 0.0
        if let crate::instruments::term_loan::types::RateSpec::Fixed { .. } = loan.rate {
            return Ok(0.0);
        }

        // Target price: quoted_clean_price% of par if set, else base PV
        let target = if let Some(px) = loan.pricing_overrides.quoted_clean_price {
            // Interpreting as % of notional_limit
            px * loan.notional_limit.amount() / 100.0
        } else {
            context.base_value.amount()
        };

        // Objective function: PV(dm) - target_price
        let objective = |dm_bp: f64| -> f64 {
            match Self::pv_given_dm(loan, &context.curves, context.as_of, dm_bp) {
                Ok(pv) => pv - target,
                Err(_) => 1e12 * dm_bp.signum(),
            }
        };

        // Solve for DM in basis points
        let solver = BrentSolver::new()
            .with_tolerance(1e-12)
            .with_initial_bracket_size(Some(50.0)); // Start with +/- 50bp bracket

        let dm_bp = solver.solve(objective, 0.0)?;

        // Return DM as decimal (bp / 10000)
        Ok(dm_bp * 1e-4)
    }
}
